//! Blockchain orchestration for minichain.
//!
//! This crate brings together all components to create a fully functional blockchain:
//! - **Consensus**: Proof of Authority block production and validation
//! - **Mempool**: Transaction pool for pending transactions
//! - **Executor**: Block execution engine
//! - **Storage**: Persistent state management
//!
//! # Example
//!
//! ```rust,no_run
//! use minichain_chain::{Blockchain, BlockchainConfig};
//! use minichain_consensus::{PoAConfig, BlockProposer};
//! use minichain_core::{Keypair, Block};
//! use minichain_storage::Storage;
//!
//! // Setup storage
//! let storage = Storage::open("./blockchain_data").unwrap();
//!
//! // Configure blockchain with PoA
//! let keypair = Keypair::generate();
//! let config = BlockchainConfig {
//!     consensus: PoAConfig::new(vec![keypair.address()], 5),
//!     max_block_size: 1000,
//! };
//!
//! // Create blockchain
//! let mut blockchain = Blockchain::new(&storage, config);
//! blockchain.register_authority(keypair.address(), keypair.public_key.clone());
//!
//! // Initialize with genesis
//! let genesis = Block::genesis(keypair.address()).signed(&keypair);
//! blockchain.init_genesis(&genesis).unwrap();
//!
//! // Submit transactions, propose blocks, etc.
//! ```

pub mod blockchain;
pub mod executor;
pub mod mempool;

// Re-export commonly used types
pub use blockchain::{Blockchain, BlockchainConfig, BlockchainError, BlockchainStats};
pub use executor::{BlockExecutionResult, ExecutionError, Executor, TransactionReceipt};
pub use mempool::{Mempool, MempoolConfig, MempoolError, MempoolStats};
