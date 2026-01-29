//! Account management command.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use minichain_core::{Address, Keypair};
use minichain_storage::{StateManager, Storage};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    command: AccountCommand,
}

#[derive(Subcommand)]
enum AccountCommand {
    /// Generate a new keypair
    New {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Name for the keypair file
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Check account balance
    Balance {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Account address (hex format)
        address: String,
    },
    /// Show account information
    Info {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Account address (hex format)
        address: String,
    },
    /// List all keypairs
    List {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,
    },
}

pub fn run(args: AccountArgs) -> Result<()> {
    match args.command {
        AccountCommand::New { data_dir, name } => new_keypair(data_dir, name),
        AccountCommand::Balance { data_dir, address } => check_balance(data_dir, address),
        AccountCommand::Info { data_dir, address } => show_info(data_dir, address),
        AccountCommand::List { data_dir } => list_keypairs(data_dir),
    }
}

fn new_keypair(data_dir: PathBuf, name: Option<String>) -> Result<()> {
    // Generate new keypair
    let keypair = Keypair::generate();
    let address = keypair.address();

    println!("{}", "Generated new keypair:".bold().cyan());
    println!();
    println!("  Address:     {}", address.to_hex().bright_yellow());
    println!(
        "  Public Key:  {}",
        hex::encode(keypair.public_key.as_bytes()).bright_black()
    );
    println!(
        "  Private Key: {}",
        hex::encode(keypair.private_key()).bright_black()
    );

    // Save to file
    let keys_dir = data_dir.join("keys");
    fs::create_dir_all(&keys_dir)?;

    let filename = if let Some(n) = name {
        format!("{}.json", n)
    } else {
        format!("account_{}.json", &address.to_hex()[2..10])
    };

    let key_file = keys_dir.join(&filename);
    let key_json = serde_json::json!({
        "address": address.to_hex(),
        "public_key": hex::encode(keypair.public_key.as_bytes()),
        "private_key": hex::encode(keypair.private_key()),
    });

    fs::write(&key_file, serde_json::to_string_pretty(&key_json)?)?;

    println!();
    println!(
        "{}  Saved to: {}",
        "✓".green().bold(),
        key_file.display().to_string().bright_black()
    );
    println!();
    println!("{}", "Keep your private key safe!".yellow().bold());

    Ok(())
}

fn check_balance(data_dir: PathBuf, address_str: String) -> Result<()> {
    let address = Address::from_hex(&address_str)
        .with_context(|| format!("Invalid address format: {}", address_str))?;

    // Open storage
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    let state = StateManager::new(&storage);
    let balance = state.get_balance(&address)?;

    println!();
    println!("  Address: {}", address.to_hex().bright_yellow());
    println!("  Balance: {}", balance.to_string().bright_cyan());
    println!();

    Ok(())
}

fn show_info(data_dir: PathBuf, address_str: String) -> Result<()> {
    let address = Address::from_hex(&address_str)
        .with_context(|| format!("Invalid address format: {}", address_str))?;

    // Open storage
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    let state = StateManager::new(&storage);
    let account = state.get_account(&address)?;

    println!();
    println!("{}", "Account Information:".bold().cyan());
    println!();
    println!("  Address:      {}", address.to_hex().bright_yellow());
    println!(
        "  Balance:      {}",
        account.balance.to_string().bright_cyan()
    );
    println!(
        "  Nonce:        {}",
        account.nonce.to_string().bright_cyan()
    );
    println!(
        "  Is Contract:  {}",
        if account.is_contract() {
            "Yes".green()
        } else {
            "No".bright_black()
        }
    );

    if let Some(code_hash) = account.code_hash {
        println!("  Code Hash:    {}", code_hash.to_hex().bright_black());
    }

    println!();

    Ok(())
}

fn list_keypairs(data_dir: PathBuf) -> Result<()> {
    let keys_dir = data_dir.join("keys");

    if !keys_dir.exists() {
        println!("{}", "No keypairs found.".yellow());
        println!(
            "Use {} to create a new keypair.",
            "minichain account new".bright_cyan()
        );
        return Ok(());
    }

    println!("{}", "Saved Keypairs:".bold().cyan());
    println!();

    let mut count = 0;
    for entry in fs::read_dir(&keys_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path)?;
            let json: serde_json::Value = serde_json::from_str(&contents)?;

            if let Some(address) = json.get("address").and_then(|v| v.as_str()) {
                count += 1;
                let filename = path.file_name().unwrap().to_string_lossy();
                println!(
                    "  {} {}",
                    format!("{}:", filename).bright_black(),
                    address.bright_yellow()
                );
            }
        }
    }

    if count == 0 {
        println!("  {}", "No keypairs found.".yellow());
    }

    println!();
    Ok(())
}
