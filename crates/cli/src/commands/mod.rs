//! CLI commands module.

use anyhow::Result;
use clap::Subcommand;

mod account;
mod block;
mod call;
mod deploy;
mod explore;
mod init;
mod tx;

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new chain
    Init,
    /// Account management
    Account,
    /// Transaction operations
    Tx,
    /// Block operations
    Block,
    /// Deploy a contract
    Deploy,
    /// Call a contract
    Call,
    /// Block explorer
    Explore,
}

pub fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Init => println!("init: not yet implemented"),
        Commands::Account => println!("account: not yet implemented"),
        Commands::Tx => println!("tx: not yet implemented"),
        Commands::Block => println!("block: not yet implemented"),
        Commands::Deploy => println!("deploy: not yet implemented"),
        Commands::Call => println!("call: not yet implemented"),
        Commands::Explore => println!("explore: not yet implemented"),
    }
    Ok(())
}
