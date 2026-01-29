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
    /// Initialize a new blockchain
    Init(init::InitArgs),
    /// Account management (create keypairs, check balances)
    Account(account::AccountArgs),
    /// Transaction operations
    Tx(tx::TxArgs),
    /// Block operations
    Block(block::BlockArgs),
    /// Deploy a contract
    Deploy(deploy::DeployArgs),
    /// Call a contract
    Call(call::CallArgs),
    /// Block explorer
    Explore,
}

pub fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Init(args) => init::run(args),
        Commands::Account(args) => account::run(args),
        Commands::Tx(args) => tx::run(args),
        Commands::Block(args) => block::run(args),
        Commands::Deploy(args) => deploy::run(args),
        Commands::Call(args) => call::run(args),
        Commands::Explore => {
            println!("explore: not yet implemented");
            Ok(())
        }
    }
}
