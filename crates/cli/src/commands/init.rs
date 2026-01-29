//! Initialize chain command.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use minichain_chain::{Blockchain, BlockchainConfig};
use minichain_consensus::PoAConfig;
use minichain_core::{Block, Keypair};
use minichain_storage::Storage;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitArgs {
    /// Directory to store blockchain data
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// Number of authorities to generate
    #[arg(short, long, default_value = "1")]
    authorities: usize,

    /// Block time in seconds
    #[arg(short, long, default_value = "5")]
    block_time: u64,
}

pub fn run(args: InitArgs) -> Result<()> {
    println!("{}", "Initializing minichain...".bold().cyan());
    println!();

    // Create data directory
    fs::create_dir_all(&args.data_dir)
        .with_context(|| format!("Failed to create data directory: {:?}", args.data_dir))?;

    // Open storage
    let storage = Storage::open(&args.data_dir).with_context(|| "Failed to open storage")?;

    println!("{}  Created data directory", "✓".green().bold());

    // Generate authority keypairs
    println!();
    println!("{}", "Generating authorities...".bold());

    let mut authorities = Vec::new();
    let mut keypairs = Vec::new();

    for i in 0..args.authorities {
        let keypair = Keypair::generate();
        let address = keypair.address();

        authorities.push(address);
        keypairs.push(keypair);

        println!(
            "  Authority {}: {}",
            i + 1,
            address.to_hex().bright_yellow()
        );
    }

    // Create blockchain config
    let config = BlockchainConfig {
        consensus: PoAConfig::new(authorities.clone(), args.block_time),
        max_block_size: 1000,
    };

    // Create blockchain
    let mut blockchain = Blockchain::new(&storage, config);

    // Register all authorities
    for keypair in &keypairs {
        blockchain.register_authority(keypair.address(), keypair.public_key.clone());
    }

    // Create and sign genesis block with first authority
    let genesis_authority = &keypairs[0];
    let genesis = Block::genesis(genesis_authority.address()).signed(genesis_authority);

    // Initialize blockchain with genesis
    blockchain
        .init_genesis(&genesis)
        .with_context(|| "Failed to initialize genesis block")?;

    println!();
    println!("{}  Created genesis block", "✓".green().bold());
    println!("    Hash: {}", genesis.hash().to_hex().bright_yellow());
    println!("    Height: {}", "0".bright_cyan());

    // Save authority keypairs to disk
    let keys_dir = args.data_dir.join("keys");
    fs::create_dir_all(&keys_dir)?;

    for (i, keypair) in keypairs.iter().enumerate() {
        let key_file = keys_dir.join(format!("authority_{}.json", i));
        let key_json = serde_json::json!({
            "address": keypair.address().to_hex(),
            "public_key": hex::encode(keypair.public_key.as_bytes()),
            "private_key": hex::encode(keypair.private_key()),
        });

        fs::write(&key_file, serde_json::to_string_pretty(&key_json)?)?;
        println!(
            "{}  Saved authority {} keypair to: {}",
            "✓".green().bold(),
            i + 1,
            key_file.display().to_string().bright_black()
        );
    }

    // Save config
    let config_file = args.data_dir.join("config.json");
    let config_json = serde_json::json!({
        "authorities": authorities.iter().map(|a| a.to_hex()).collect::<Vec<_>>(),
        "block_time": args.block_time,
        "max_block_size": 1000,
    });

    fs::write(&config_file, serde_json::to_string_pretty(&config_json)?)?;
    println!(
        "{}  Saved config to: {}",
        "✓".green().bold(),
        config_file.display().to_string().bright_black()
    );

    println!();
    println!("{}", "Chain initialized successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!(
        "  • Use {} to create accounts",
        "minichain account new".bright_cyan()
    );
    println!(
        "  • Use {} to send transactions",
        "minichain tx send".bright_cyan()
    );
    println!(
        "  • Use {} to explore blocks",
        "minichain block list".bright_cyan()
    );

    Ok(())
}
