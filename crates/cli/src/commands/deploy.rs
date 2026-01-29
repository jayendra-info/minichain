//! Deploy contract command.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use minichain_assembler::assemble;
use minichain_chain::{Blockchain, BlockchainConfig};
use minichain_consensus::PoAConfig;
use minichain_core::{Address, Keypair, Transaction};
use minichain_storage::Storage;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct DeployArgs {
    /// Directory to store blockchain data
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// Account to deploy from (keypair file without .json extension)
    #[arg(short, long)]
    from: String,

    /// Path to assembly source file
    #[arg(short, long)]
    source: PathBuf,

    /// Gas price
    #[arg(long, default_value = "1")]
    gas_price: u64,
}

pub fn run(args: DeployArgs) -> Result<()> {
    println!("{}", "Deploying contract...".bold().cyan());
    println!();

    // Read and compile assembly
    let source_code = fs::read_to_string(&args.source)
        .with_context(|| format!("Failed to read source file: {:?}", args.source))?;

    println!(
        "  Compiling: {}",
        args.source.display().to_string().bright_black()
    );

    let bytecode = assemble(&source_code).with_context(|| "Failed to compile assembly code")?;

    println!(
        "{}  Compiled to {} bytes",
        "✓".green().bold(),
        bytecode.len()
    );
    println!();

    // Load deployer keypair
    let keys_dir = args.data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, &args.from)?;
    let from = keypair.address();

    // Open storage and get nonce
    let storage = Storage::open(&args.data_dir).with_context(|| "Failed to open storage")?;

    let state = minichain_storage::StateManager::new(&storage);
    let nonce = state.get_nonce(&from)?;
    let balance = state.get_balance(&from)?;

    println!("  Deployer:  {}", from.to_hex().bright_yellow());
    println!("  Nonce:     {}", nonce.to_string().bright_black());
    println!("  Balance:   {}", balance.to_string().bright_black());
    println!();

    // Check balance (approximate gas needed)
    let gas_limit = 21_000 + (bytecode.len() as u64 * 200);
    let total_cost = gas_limit * args.gas_price;

    if balance < total_cost {
        anyhow::bail!(
            "Insufficient balance: have {}, need {} (estimated)",
            balance,
            total_cost
        );
    }

    // Create and sign deploy transaction
    let tx = Transaction::deploy(from, bytecode.clone(), nonce, gas_limit, args.gas_price)
        .signed(&keypair);
    let tx_hash = tx.hash();

    // Calculate contract address
    let contract_address = tx
        .contract_address()
        .expect("deploy transaction must have contract address");

    println!("{}  Transaction created", "✓".green().bold());
    println!("    Hash: {}", tx_hash.to_hex().bright_yellow());
    println!();

    // Load config and create blockchain
    let config = load_config(&args.data_dir)?;
    let mut blockchain = Blockchain::new(&storage, config);

    // Register authorities
    register_authorities(&mut blockchain, &args.data_dir)?;

    // Submit transaction
    blockchain
        .submit_transaction(tx)
        .context("Failed to submit transaction")?;

    println!("{}  Contract deployment submitted", "✓".green().bold());
    println!();
    println!(
        "  Contract Address: {}",
        contract_address.to_hex().bright_yellow()
    );
    println!();
    println!("Transaction will be included in the next block.");
    println!(
        "Use {} to produce a block.",
        "minichain block produce".bright_cyan()
    );

    Ok(())
}

fn load_keypair(keys_dir: &Path, name: &str) -> Result<Keypair> {
    let key_file = keys_dir.join(format!("{}.json", name));
    if !key_file.exists() {
        anyhow::bail!(
            "Keypair file not found: {}. Use 'minichain account new' to create one.",
            key_file.display()
        );
    }

    let contents = fs::read_to_string(&key_file)?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;

    let private_key_hex = json
        .get("private_key")
        .and_then(|v| v.as_str())
        .context("Missing private_key in keypair file")?;

    let private_key_bytes = hex::decode(private_key_hex).context("Invalid private key hex")?;

    if private_key_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid private key length: expected 32 bytes, got {}",
            private_key_bytes.len()
        );
    }

    let mut private_key = [0u8; 32];
    private_key.copy_from_slice(&private_key_bytes);

    Keypair::from_private_key(&private_key).context("Failed to create keypair from private key")
}

// Helper function to load blockchain config
fn load_config(data_dir: &Path) -> Result<BlockchainConfig> {
    let config_file = data_dir.join("config.json");
    let contents = fs::read_to_string(&config_file)
        .context("Failed to read config.json. Did you run 'minichain init'?")?;

    let json: serde_json::Value = serde_json::from_str(&contents)?;

    let authorities: Vec<Address> = json
        .get("authorities")
        .and_then(|v| v.as_array())
        .context("Missing authorities in config")?
        .iter()
        .map(|v| {
            v.as_str()
                .and_then(|s| Address::from_hex(s).ok())
                .context("Invalid authority address")
        })
        .collect::<Result<Vec<_>>>()?;

    let block_time = json.get("block_time").and_then(|v| v.as_u64()).unwrap_or(5);

    let max_block_size = json
        .get("max_block_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000) as usize;

    Ok(BlockchainConfig {
        consensus: PoAConfig::new(authorities, block_time),
        max_block_size,
    })
}

// Helper function to register authorities
fn register_authorities(blockchain: &mut Blockchain, data_dir: &Path) -> Result<()> {
    let keys_dir = data_dir.join("keys");

    for entry in fs::read_dir(&keys_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("authority_"))
        {
            let contents = fs::read_to_string(&path)?;
            let json: serde_json::Value = serde_json::from_str(&contents)?;

            let address_hex = json
                .get("address")
                .and_then(|v| v.as_str())
                .context("Missing address in authority file")?;
            let pubkey_hex = json
                .get("public_key")
                .and_then(|v| v.as_str())
                .context("Missing public_key in authority file")?;

            let address = Address::from_hex(address_hex)?;
            let pubkey_bytes = hex::decode(pubkey_hex)?;

            if pubkey_bytes.len() != 32 {
                continue;
            }

            let mut pubkey_arr = [0u8; 32];
            pubkey_arr.copy_from_slice(&pubkey_bytes);

            // Create public key from bytes
            let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pubkey_arr)
                .context("Invalid public key")?;
            let public_key = minichain_core::PublicKey(verifying_key);

            blockchain.register_authority(address, public_key);
        }
    }

    Ok(())
}
