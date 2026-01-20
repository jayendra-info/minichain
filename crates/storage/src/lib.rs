//! Persistent storage layer for minichain.
//!
//! This crate provides the storage backend for the blockchain:
//! - Account state (balances, nonces, contract code)
//! - Block storage (by hash and height)
//! - Contract storage (SLOAD/SSTORE)
//! - State root computation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Application Layer                     │
//! │            (Chain, VM, Transaction Execution)            │
//! └────────────────────────┬────────────────────────────────┘
//!                          │
//! ┌────────────────────────▼────────────────────────────────┐
//! │                   Storage Layer                          │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
//! │  │ StateManager│  │ ChainStore  │  │ Storage (DB)    │  │
//! │  │  - Accounts │  │  - Blocks   │  │  - sled wrapper │  │
//! │  │  - Balances │  │  - Height   │  │  - serialization│  │
//! │  │  - Contract │  │  - Genesis  │  │  - key helpers  │  │
//! │  │    Storage  │  │             │  │                 │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────┘  │
//! └────────────────────────┬────────────────────────────────┘
//!                          │
//! ┌────────────────────────▼────────────────────────────────┐
//! │                    sled Database                         │
//! │              (Embedded Key-Value Store)                  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use minichain_storage::{Storage, StateManager, ChainStore};
//! use minichain_core::{Address, Block};
//!
//! // Open database
//! let storage = Storage::open("./blockchain_data").unwrap();
//!
//! // Work with accounts
//! let state = StateManager::new(&storage);
//! let alice = Address([0xAA; 20]);
//! state.set_balance(&alice, 1_000_000).unwrap();
//!
//! // Work with blocks
//! let chain = ChainStore::new(&storage);
//! let genesis = Block::genesis(alice);
//! chain.init_genesis(&genesis).unwrap();
//! ```

pub mod chain;
pub mod db;
pub mod state;

// Re-export commonly used types
pub use chain::ChainStore;
pub use db::{BatchOp, Result, Storage, StorageError};
pub use state::{StateManager, StorageSlot};
