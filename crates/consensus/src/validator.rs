//! Transaction and block validation rules.
//!
//! This module validates transactions and blocks according to consensus rules.

use minichain_core::{Block, Hash, PublicKey, Transaction};
use thiserror::Error;

/// Errors that can occur during validation.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("transaction signature verification failed")]
    InvalidSignature,

    #[error("transaction nonce mismatch (expected {expected}, got {got})")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("insufficient balance (required {required}, available {available})")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("gas limit too low (minimum {minimum}, got {got})")]
    GasLimitTooLow { minimum: u64, got: u64 },

    #[error("gas price is zero")]
    ZeroGasPrice,

    #[error("empty transaction data for contract deployment")]
    EmptyDeploymentData,

    #[error("block height mismatch (expected {expected}, got {got})")]
    InvalidHeight { expected: u64, got: u64 },

    #[error("block prev_hash mismatch")]
    InvalidPrevHash,

    #[error("block merkle root verification failed")]
    InvalidMerkleRoot,

    #[error("duplicate transaction in block")]
    DuplicateTransaction,

    #[error("transaction hash mismatch")]
    TransactionHashMismatch,

    #[error("block contains no transactions and is not genesis")]
    EmptyBlock,
}

pub type Result<T> = std::result::Result<T, ValidationError>;

/// Transaction validator.
pub struct TransactionValidator;

impl TransactionValidator {
    /// Basic transaction validation (format, signature, etc).
    ///
    /// Note: This does NOT verify the signature since Ed25519 requires
    /// the public key which must be obtained from the blockchain state.
    /// Use `validate_with_signature` for full validation including signature.
    pub fn validate_transaction(tx: &Transaction) -> Result<()> {
        // Check gas price
        if tx.gas_price == 0 {
            return Err(ValidationError::ZeroGasPrice);
        }

        // Check gas limit minimums
        let min_gas = if tx.is_deploy() {
            21_000 // Minimum gas for contract deployment
        } else if tx.is_call() {
            21_000 + tx.data.len() as u64 * 68 // Base + calldata cost
        } else {
            21_000 // Minimum gas for transfer
        };

        if tx.gas_limit < min_gas {
            return Err(ValidationError::GasLimitTooLow {
                minimum: min_gas,
                got: tx.gas_limit,
            });
        }

        // Check deployment has bytecode
        if tx.is_deploy() && tx.data.is_empty() {
            return Err(ValidationError::EmptyDeploymentData);
        }

        Ok(())
    }

    /// Validate transaction with signature verification.
    pub fn validate_with_signature(tx: &Transaction, public_key: &PublicKey) -> Result<()> {
        // Verify signature first
        tx.verify(public_key)
            .map_err(|_| ValidationError::InvalidSignature)?;

        // Then validate format
        Self::validate_transaction(tx)?;

        Ok(())
    }

    /// Validate transaction against account state.
    pub fn validate_against_state(
        tx: &Transaction,
        sender_nonce: u64,
        sender_balance: u64,
    ) -> Result<()> {
        // Check nonce
        if tx.nonce != sender_nonce {
            return Err(ValidationError::InvalidNonce {
                expected: sender_nonce,
                got: tx.nonce,
            });
        }

        // Check balance (value + max gas cost)
        let max_cost = tx.max_cost();
        if sender_balance < max_cost {
            return Err(ValidationError::InsufficientBalance {
                required: max_cost,
                available: sender_balance,
            });
        }

        Ok(())
    }

    /// Full transaction validation (signature + format + state checks).
    pub fn validate_full(
        tx: &Transaction,
        public_key: &PublicKey,
        sender_nonce: u64,
        sender_balance: u64,
    ) -> Result<()> {
        Self::validate_with_signature(tx, public_key)?;
        Self::validate_against_state(tx, sender_nonce, sender_balance)?;
        Ok(())
    }
}

/// Block validator.
pub struct BlockValidator;

impl BlockValidator {
    /// Validate block structure and contents.
    pub fn validate_block_structure(block: &Block) -> Result<()> {
        // Verify merkle root
        if !block.verify_merkle_root() {
            return Err(ValidationError::InvalidMerkleRoot);
        }

        // Check for duplicate transactions
        let mut seen = std::collections::HashSet::new();
        for tx in &block.transactions {
            let hash = tx.hash();
            if !seen.insert(hash) {
                return Err(ValidationError::DuplicateTransaction);
            }
        }

        // Non-genesis blocks should have transactions (optional check)
        // Note: We allow empty blocks for PoA to maintain liveness
        // Uncomment if you want to enforce non-empty blocks:
        // if !block.is_genesis() && block.transactions.is_empty() {
        //     return Err(ValidationError::EmptyBlock);
        // }

        Ok(())
    }

    /// Validate block extends the parent correctly.
    pub fn validate_block_extends_parent(
        block: &Block,
        parent_hash: Hash,
        parent_height: u64,
    ) -> Result<()> {
        // Check height
        if block.header.height != parent_height + 1 {
            return Err(ValidationError::InvalidHeight {
                expected: parent_height + 1,
                got: block.header.height,
            });
        }

        // Check prev_hash
        if block.header.prev_hash != parent_hash {
            return Err(ValidationError::InvalidPrevHash);
        }

        Ok(())
    }

    /// Validate all transactions in the block (format only, not signatures).
    ///
    /// Note: This does NOT verify transaction signatures since that requires
    /// public keys from the blockchain state. Signature verification should
    /// be done during transaction execution.
    pub fn validate_block_transactions(block: &Block) -> Result<()> {
        for tx in &block.transactions {
            TransactionValidator::validate_transaction(tx)?;
        }
        Ok(())
    }

    /// Full block validation (structure + parent + transactions).
    pub fn validate_full(block: &Block, parent_hash: Hash, parent_height: u64) -> Result<()> {
        Self::validate_block_structure(block)?;
        Self::validate_block_extends_parent(block, parent_hash, parent_height)?;
        Self::validate_block_transactions(block)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minichain_core::{Address, Keypair};

    #[test]
    fn test_valid_transfer_transaction() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        assert!(TransactionValidator::validate_transaction(&tx).is_ok());
    }

    #[test]
    fn test_valid_deployment_transaction() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let bytecode = vec![0x01, 0x02, 0x03];

        let tx = Transaction::deploy(from, bytecode, 0, 100_000, 1).signed(&keypair);

        assert!(TransactionValidator::validate_transaction(&tx).is_ok());
    }

    #[test]
    fn test_invalid_signature_rejected() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let to = Address::from_bytes([2u8; 20]);

        // Sign with keypair1 but send from keypair2's address
        let mut tx = Transaction::transfer(keypair2.address(), to, 1000, 0, 1);
        tx.sign(&keypair1);

        // Try to validate with keypair2's public key (should fail)
        assert!(matches!(
            TransactionValidator::validate_with_signature(&tx, &keypair2.public_key),
            Err(ValidationError::InvalidSignature)
        ));

        // Validate with keypair1's public key (should also fail because from != keypair1.address())
        // Actually, the signature itself is valid for the tx hash, but logically wrong
        // The signature verification will pass, but in real use the address wouldn't match
    }

    #[test]
    fn test_zero_gas_price_rejected() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::new(0, from, Some(to), 1000, vec![], 21_000, 0).signed(&keypair);

        assert!(matches!(
            TransactionValidator::validate_transaction(&tx),
            Err(ValidationError::ZeroGasPrice)
        ));
    }

    #[test]
    fn test_gas_limit_too_low_rejected() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::new(0, from, Some(to), 1000, vec![], 10_000, 1).signed(&keypair);

        assert!(matches!(
            TransactionValidator::validate_transaction(&tx),
            Err(ValidationError::GasLimitTooLow { .. })
        ));
    }

    #[test]
    fn test_empty_deployment_rejected() {
        let keypair = Keypair::generate();
        let from = keypair.address();

        let tx = Transaction::new(0, from, None, 0, vec![], 100_000, 1).signed(&keypair);

        assert!(matches!(
            TransactionValidator::validate_transaction(&tx),
            Err(ValidationError::EmptyDeploymentData)
        ));
    }

    #[test]
    fn test_validate_against_state_nonce_check() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 5, 1).signed(&keypair);

        // Nonce mismatch
        assert!(matches!(
            TransactionValidator::validate_against_state(&tx, 0, 100_000),
            Err(ValidationError::InvalidNonce { .. })
        ));

        // Correct nonce
        assert!(TransactionValidator::validate_against_state(&tx, 5, 100_000).is_ok());
    }

    #[test]
    fn test_validate_against_state_balance_check() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::new(0, from, Some(to), 1000, vec![], 21_000, 1).signed(&keypair);

        let max_cost = tx.max_cost(); // 1000 + 21_000 * 1 = 22_000

        // Insufficient balance
        assert!(matches!(
            TransactionValidator::validate_against_state(&tx, 0, 10_000),
            Err(ValidationError::InsufficientBalance { .. })
        ));

        // Sufficient balance
        assert!(TransactionValidator::validate_against_state(&tx, 0, max_cost).is_ok());
    }

    #[test]
    fn test_block_merkle_root_validation() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let block = Block::new(1, Hash::ZERO, vec![tx], Hash::ZERO, from);

        assert!(BlockValidator::validate_block_structure(&block).is_ok());

        // Tamper with merkle root
        let mut bad_block = block.clone();
        bad_block.header.merkle_root = Hash::ZERO;

        assert!(matches!(
            BlockValidator::validate_block_structure(&bad_block),
            Err(ValidationError::InvalidMerkleRoot)
        ));
    }

    #[test]
    fn test_block_duplicate_transaction_rejected() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        // Create block with duplicate transaction
        let block = Block::new(
            1,
            Hash::ZERO,
            vec![tx.clone(), tx.clone()],
            Hash::ZERO,
            from,
        );

        assert!(matches!(
            BlockValidator::validate_block_structure(&block),
            Err(ValidationError::DuplicateTransaction)
        ));
    }

    #[test]
    fn test_block_extends_parent() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let parent_hash = Hash::from_bytes([0xAA; 32]);

        let block = Block::new(5, parent_hash, vec![], Hash::ZERO, from);

        // Valid extension
        assert!(BlockValidator::validate_block_extends_parent(&block, parent_hash, 4).is_ok());

        // Invalid height
        assert!(matches!(
            BlockValidator::validate_block_extends_parent(&block, parent_hash, 5),
            Err(ValidationError::InvalidHeight { .. })
        ));

        // Invalid prev_hash
        assert!(matches!(
            BlockValidator::validate_block_extends_parent(&block, Hash::ZERO, 4),
            Err(ValidationError::InvalidPrevHash)
        ));
    }

    #[test]
    fn test_full_block_validation() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);
        let parent_hash = Hash::from_bytes([0xAA; 32]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let block = Block::new(5, parent_hash, vec![tx], Hash::ZERO, from);

        assert!(BlockValidator::validate_full(&block, parent_hash, 4).is_ok());
    }
}
