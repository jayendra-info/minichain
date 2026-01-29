//! Block operations command.

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use minichain_chain::{Blockchain, BlockchainConfig};
use minichain_consensus::{BlockProposer, PoAConfig};
use minichain_core::{Address, Keypair};
use minichain_storage::Storage;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct BlockArgs {
    #[command(subcommand)]
    command: BlockCommand,
}

#[derive(Subcommand)]
enum BlockCommand {
    /// List recent blocks
    List {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Number of blocks to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Show detailed block information
    Info {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Block number or hash (hex format)
        block_id: String,
    },
    /// Produce a new block (authority only)
    Produce {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Authority name (keypair file without .json extension)
        #[arg(short, long)]
        authority: String,
    },
}

pub fn run(args: BlockArgs) -> Result<()> {
    match args.command {
        BlockCommand::List { data_dir, count } => list_blocks(data_dir, count),
        BlockCommand::Info { data_dir, block_id } => show_block_info(data_dir, block_id),
        BlockCommand::Produce {
            data_dir,
            authority,
        } => produce_block(data_dir, authority),
    }
}

fn list_blocks(data_dir: PathBuf, count: usize) -> Result<()> {
    // Open storage
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    let chain = minichain_storage::ChainStore::new(&storage);
    let head_height = chain.get_height()?;

    println!();
    println!("{}", "Recent Blocks:".bold().cyan());
    println!();

    let start_height = if head_height >= count as u64 {
        head_height - count as u64 + 1
    } else {
        0
    };

    for height in (start_height..=head_height).rev() {
        let block = chain
            .get_block_by_height(height)?
            .context("Block not found")?;

        println!(
            "  {} {} {}",
            format!("#{}", height).bright_black(),
            block.hash().to_hex()[..16].bright_yellow(),
            format!("({} txs)", block.transactions.len()).bright_black()
        );
    }

    println!();
    Ok(())
}

fn show_block_info(data_dir: PathBuf, block_id: String) -> Result<()> {
    // Open storage
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    let chain = minichain_storage::ChainStore::new(&storage);

    // Try parsing as height first, then as hash
    let block = if let Ok(height) = block_id.parse::<u64>() {
        chain
            .get_block_by_height(height)?
            .context("Block not found")?
    } else {
        // Try as hex hash
        let hash = minichain_core::Hash::from_hex(&block_id)
            .with_context(|| format!("Invalid block hash: {}", block_id))?;
        chain.get_block_by_hash(&hash)?.context("Block not found")?
    };

    let block_hash = block.hash();
    let height = block.header.height;

    println!();
    println!("{}", "Block Information:".bold().cyan());
    println!();
    println!("  Height:       {}", height.to_string().bright_cyan());
    println!("  Hash:         {}", block_hash.to_hex().bright_yellow());
    println!(
        "  Parent Hash:  {}",
        block.header.prev_hash.to_hex().bright_black()
    );
    println!(
        "  State Root:   {}",
        block.header.state_root.to_hex().bright_black()
    );
    println!(
        "  Timestamp:    {}",
        block.header.timestamp.to_string().bright_black()
    );
    println!(
        "  Transactions: {}",
        block.transactions.len().to_string().bright_cyan()
    );
    println!();

    if !block.transactions.is_empty() {
        println!("{}", "Transactions:".bold());
        println!();
        for (i, tx) in block.transactions.iter().enumerate() {
            let tx_hash = tx.hash();
            println!(
                "  {} {}",
                format!("{}.", i + 1).bright_black(),
                tx_hash.to_hex()[..16].bright_yellow()
            );
        }
        println!();
    }

    Ok(())
}

fn produce_block(data_dir: PathBuf, authority_name: String) -> Result<()> {
    println!("{}", "Producing new block...".bold().cyan());
    println!();

    // Load authority keypair
    let keys_dir = data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, &authority_name)?;
    let authority_addr = keypair.address();

    println!("  Authority: {}", authority_addr.to_hex().bright_yellow());

    // Open storage
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    // Load config and create blockchain
    let config = load_config(&data_dir)?;

    // Check if this keypair is actually an authority
    if !config.consensus.authorities.contains(&authority_addr) {
        bail!(
            "Address {} is not an authority. Authorities: {:?}",
            authority_addr.to_hex(),
            config
                .consensus
                .authorities
                .iter()
                .map(|a| a.to_hex())
                .collect::<Vec<_>>()
        );
    }

    // Clone consensus config before moving config into blockchain
    let consensus_config = config.consensus.clone();
    let mut blockchain = Blockchain::new(&storage, config);

    // Register all authorities
    register_authorities(&mut blockchain, &data_dir)?;

    // Get current chain state
    let chain = minichain_storage::ChainStore::new(&storage);
    let head_height = chain.get_height()?;

    println!(
        "  Current Height: {}",
        head_height.to_string().bright_black()
    );

    // Create block proposer and produce block
    let proposer = BlockProposer::new(keypair, consensus_config);
    let block = blockchain
        .propose_block(&proposer)
        .context("Failed to produce block")?;

    let block_hash = block.hash();
    let new_height = head_height + 1;

    println!();
    println!("{}  Block produced", "✓".green().bold());
    println!("    Hash:   {}", block_hash.to_hex().bright_yellow());
    println!("    Height: {}", new_height.to_string().bright_cyan());
    println!(
        "    Txs:    {}",
        block.transactions.len().to_string().bright_cyan()
    );
    println!();

    Ok(())
}

fn load_keypair(keys_dir: &Path, name: &str) -> Result<Keypair> {
    let key_file = keys_dir.join(format!("{}.json", name));
    if !key_file.exists() {
        bail!(
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
        bail!(
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
