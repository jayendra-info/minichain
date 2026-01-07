//! Core blockchain primitives for minichain.
//!
//! This crate provides the fundamental types used throughout the blockchain:
//! - Cryptographic primitives (hashing, signing, addresses)
//! - Account state
//! - Transactions
//! - Blocks and block headers
//! - Merkle trees

pub mod account;
pub mod block;
pub mod crypto;
pub mod hash;
pub mod merkle;
pub mod transaction;

// Re-export commonly used types at the crate root
pub use account::Account;
pub use block::{Block, BlockHeader};
pub use crypto::{Address, CryptoError, Keypair, PublicKey, Signature};
pub use hash::{hash, hash_concat, Hash, H256};
pub use merkle::{merkle_root, MerkleProof, MerkleTree};
pub use transaction::{Transaction, TransactionError};
