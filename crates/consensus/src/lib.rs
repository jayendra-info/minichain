//! Proof of Authority consensus for minichain.
//!
//! This crate provides a complete PoA consensus implementation including:
//! - Authority management and round-robin block production
//! - Transaction validation (signature, nonce, balance checks)
//! - Block validation (structure, merkle proofs, parent links)
//! - Block proposing and signing
//!
//! # Example
//!
//! ```rust,no_run
//! use minichain_consensus::{PoAConfig, Authority, BlockProposer};
//! use minichain_core::{Keypair, Block, Hash};
//!
//! // Setup authorities
//! let keypair1 = Keypair::generate();
//! let keypair2 = Keypair::generate();
//! let config = PoAConfig::new(vec![keypair1.address(), keypair2.address()], 5);
//!
//! // Create authority manager
//! let mut authority = Authority::new(config.clone());
//! authority.register_public_key(keypair1.address(), keypair1.public_key.clone());
//! authority.register_public_key(keypair2.address(), keypair2.public_key.clone());
//!
//! // Propose a block
//! let proposer = BlockProposer::new(keypair1, config);
//! let block = proposer.propose_block(
//!     0,
//!     Hash::ZERO,
//!     vec![],
//!     Hash::ZERO,
//! ).unwrap();
//!
//! // Verify the block
//! authority.verify_block(&block, 0).unwrap();
//! ```

pub mod poa;
pub mod validator;

// Re-export commonly used types
pub use poa::{Authority, BlockProposer, ConsensusError, PoAConfig};
pub use validator::{BlockValidator, TransactionValidator, ValidationError};
