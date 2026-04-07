//! Block explorer command - Shows confirmed transactions.

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use minichain_core::Hash;
use minichain_storage::{ChainStore, Storage};
use std::path::PathBuf;

#[derive(Args)]
pub struct ExploreArgs {
    #[command(subcommand)]
    command: Option<ExploreCommand>,
}

#[derive(Subcommand)]
enum ExploreCommand {
    /// Show transaction details by hash
    Tx {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Transaction hash (hex format)
        tx_hash: String,
    },
    /// Show recent transactions across all blocks
    Transactions {
        /// Directory to store blockchain data
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        /// Number of transactions to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
}

pub fn run(args: ExploreArgs) -> Result<()> {
    match args.command {
        Some(cmd) => run_command(cmd),
        None => run_interactive(),
    }
}

fn run_command(cmd: ExploreCommand) -> Result<()> {
    match cmd {
        ExploreCommand::Tx { data_dir, tx_hash } => show_transaction(data_dir, tx_hash),
        ExploreCommand::Transactions { data_dir, count } => list_transactions(data_dir, count),
    }
}

fn run_interactive() -> Result<()> {
    println!("{}", "Block Explorer".bold().cyan());
    println!();
    println!("Available commands:");
    println!("  explore tx <hash>        - Show transaction details");
    println!("  explore transactions      - List recent transactions");
    println!();
    println!("Examples:");
    println!("  explore transactions --count 20");
    println!("  explore tx 0abc123...");
    Ok(())
}

fn show_transaction(data_dir: PathBuf, tx_hash_str: String) -> Result<()> {
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    let chain = ChainStore::new(&storage);
    let head_height = chain.get_height()?;

    let tx_hash = Hash::from_hex(&tx_hash_str)
        .with_context(|| format!("Invalid transaction hash: {}", tx_hash_str))?;

    for height in 0..=head_height {
        let block = match chain.get_block_by_height(height)? {
            Some(b) => b,
            None => continue,
        };

        for (i, tx) in block.transactions.iter().enumerate() {
            if tx.hash() == tx_hash {
                println!();
                println!("{}", "Transaction Details:".bold().cyan());
                println!("{}", "═".repeat(60).cyan());
                println!();
                println!("  {:20} {}", "Hash:".bold(), tx_hash.to_hex().yellow());
                println!("  {:20} {}", "From:".bold(), tx.from.to_hex());
                println!(
                    "  {:20} {}",
                    "To:".bold(),
                    tx.to
                        .map(|a| a.to_hex())
                        .unwrap_or_else(|| "Contract Deploy".cyan().to_string())
                );
                println!("  {:20} {}", "Value:".bold(), tx.value);
                println!("  {:20} {}", "Nonce:".bold(), tx.nonce);
                println!("  {:20} {}", "Gas Limit:".bold(), tx.gas_limit);
                println!("  {:20} {}", "Gas Price:".bold(), tx.gas_price);
                println!();
                println!(
                    "  {:20} {}",
                    "Included in:".bold(),
                    format!("Block #{}", height).cyan()
                );
                println!("  {:20} {}", "Tx Index:".bold(), i);
                println!();

                if !tx.data.is_empty() {
                    println!("  {:20}", "Calldata:".bold());
                    let hex_str = hex::encode(&tx.data);
                    let display = if hex_str.len() > 80 {
                        format!("{}...", &hex_str[..80])
                    } else {
                        hex_str
                    };
                    println!("      {}", display.black());
                    println!();
                }

                return Ok(());
            }
        }
    }

    bail!("Transaction not found: {}", tx_hash_str)
}

fn list_transactions(data_dir: PathBuf, count: usize) -> Result<()> {
    let storage = Storage::open(&data_dir)
        .with_context(|| "Failed to open storage. Did you run 'minichain init'?")?;

    let chain = ChainStore::new(&storage);
    let head_height = chain.get_height()?;

    println!();
    println!("{}", "Recent Transactions:".bold().cyan());
    println!("{}", "═".repeat(70).cyan());
    println!();
    println!(
        "  {:>6} {:<18} {:<10} {:<10} {}",
        "Block".bold(),
        "Tx Hash".bold(),
        "From".bold(),
        "To".bold(),
        "Value".bold()
    );
    println!("{}", "─".repeat(70).cyan());

    let mut txs: Vec<(u64, minichain_core::Transaction)> = Vec::new();

    for height in (0..=head_height).rev() {
        if let Some(block) = chain.get_block_by_height(height)? {
            for tx in block.transactions {
                txs.push((height, tx));
                if txs.len() >= count {
                    break;
                }
            }
        }
        if txs.len() >= count {
            break;
        }
    }

    for (height, tx) in &txs {
        let from = tx.from.to_hex();
        let to = tx
            .to
            .map(|a| a.to_hex())
            .unwrap_or_else(|| "Contract Deploy".to_string());

        println!(
            "  {:>6} {} {} {} {}",
            height.to_string().cyan(),
            tx.hash().to_hex().yellow(),
            from,
            to,
            tx.value
        );
    }

    if txs.is_empty() {
        println!("  (no transactions found)");
    }

    println!();
    Ok(())
}
