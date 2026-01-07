//! minichain CLI entry point.

use clap::Parser;

mod commands;
mod repl;

#[derive(Parser)]
#[command(name = "minichain")]
#[command(about = "A minimal blockchain implementation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<commands::Commands>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => {
            if let Err(e) = commands::run(cmd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            println!("minichain - A minimal blockchain implementation");
            println!("Run 'minichain --help' for usage information.");
        }
    }
}
