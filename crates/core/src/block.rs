//! Block and block header structures.

use crate::crypto::{Address, Keypair, Signature};
use crate::hash::{hash, Hash};
use crate::merkle::merkle_root;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// The header of a block containing metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block height (0 for genesis).
    pub height: u64,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
    /// Hash of the previous block.
    pub prev_hash: Hash,
    /// Merkle root of transactions.
    pub merkle_root: Hash,
    /// Root hash of the world state after applying this block.
    pub state_root: Hash,
    /// Address of the block author (PoA authority).
    pub author: Address,
    /// Difficulty (always 1 for PoA).
    pub difficulty: u64,
    /// Nonce (unused in PoA, kept for structure).
    pub nonce: u64,
}

impl BlockHeader {
    /// Calculate the hash of this block header.
    pub fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).expect("serialization should not fail");
        hash(&encoded)
    }

    /// Get the current Unix timestamp.
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs()
    }
}

/// A complete block including header, transactions, and signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Block header.
    pub header: BlockHeader,
    /// List of transactions in this block.
    pub transactions: Vec<Transaction>,
    /// Authority signature over the block header.
    pub signature: Signature,
}

impl Block {
    /// Create a new unsigned block.
    pub fn new(
        height: u64,
        prev_hash: Hash,
        transactions: Vec<Transaction>,
        state_root: Hash,
        author: Address,
    ) -> Self {
        let tx_hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();
        let merkle_root = merkle_root(&tx_hashes);

        Self {
            header: BlockHeader {
                height,
                timestamp: BlockHeader::current_timestamp(),
                prev_hash,
                merkle_root,
                state_root,
                author,
                difficulty: 1,
                nonce: 0,
            },
            transactions,
            signature: Signature::default(),
        }
    }

    /// Create the genesis block.
    pub fn genesis(authority: Address) -> Self {
        Self {
            header: BlockHeader {
                height: 0,
                timestamp: BlockHeader::current_timestamp(),
                prev_hash: Hash::ZERO,
                merkle_root: Hash::ZERO,
                state_root: Hash::ZERO,
                author: authority,
                difficulty: 1,
                nonce: 0,
            },
            transactions: Vec::new(),
            signature: Signature::default(),
        }
    }

    /// Get the block hash (hash of the header).
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Get the block height.
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Check if this is the genesis block.
    pub fn is_genesis(&self) -> bool {
        self.header.height == 0 && self.header.prev_hash == Hash::ZERO
    }

    /// Get the number of transactions in this block.
    pub fn tx_count(&self) -> usize {
        self.transactions.len()
    }

    /// Sign the block with the authority's keypair.
    pub fn sign(&mut self, keypair: &Keypair) {
        let hash = self.header.hash();
        self.signature = keypair.sign_hash(&hash);
    }

    /// Create a signed block.
    pub fn signed(mut self, keypair: &Keypair) -> Self {
        self.sign(keypair);
        self
    }

    /// Verify the block signature against the author's public key.
    pub fn verify_signature(&self, public_key: &crate::crypto::PublicKey) -> bool {
        let hash = self.header.hash();
        public_key.verify(hash.as_bytes(), &self.signature).is_ok()
    }

    /// Verify the merkle root matches the transactions.
    pub fn verify_merkle_root(&self) -> bool {
        let tx_hashes: Vec<Hash> = self.transactions.iter().map(|tx| tx.hash()).collect();
        let computed = merkle_root(&tx_hashes);
        computed == self.header.merkle_root
    }

    /// Calculate the total gas used by all transactions (simplified).
    pub fn total_gas_limit(&self) -> u64 {
        self.transactions.iter().map(|tx| tx.gas_limit).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let authority = Address::from_bytes([1u8; 20]);
        let genesis = Block::genesis(authority);

        assert!(genesis.is_genesis());
        assert_eq!(genesis.height(), 0);
        assert_eq!(genesis.header.prev_hash, Hash::ZERO);
        assert_eq!(genesis.header.author, authority);
        assert!(genesis.transactions.is_empty());
    }

    #[test]
    fn test_block_hash_deterministic() {
        let authority = Address::from_bytes([1u8; 20]);
        let block = Block::genesis(authority);

        let h1 = block.hash();
        let h2 = block.hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_block_signing() {
        let keypair = Keypair::generate();
        let block = Block::genesis(keypair.address()).signed(&keypair);

        assert!(block.verify_signature(&keypair.public_key));
    }

    #[test]
    fn test_wrong_key_fails_verification() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();

        let block = Block::genesis(keypair1.address()).signed(&keypair1);

        assert!(!block.verify_signature(&keypair2.public_key));
    }

    #[test]
    fn test_block_with_transactions() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 100, 0, 1).signed(&keypair);

        let block = Block::new(1, Hash::ZERO, vec![tx], Hash::ZERO, from);

        assert_eq!(block.tx_count(), 1);
        assert!(block.verify_merkle_root());
    }

    #[test]
    fn test_merkle_root_verification() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx1 = Transaction::transfer(from, to, 100, 0, 1).signed(&keypair);
        let tx2 = Transaction::transfer(from, to, 200, 1, 1).signed(&keypair);

        let block = Block::new(1, Hash::ZERO, vec![tx1, tx2], Hash::ZERO, from);

        assert!(block.verify_merkle_root());
    }

    #[test]
    fn test_empty_block_merkle_root() {
        let authority = Address::from_bytes([1u8; 20]);
        let block = Block::new(1, Hash::ZERO, vec![], Hash::ZERO, authority);

        assert!(block.verify_merkle_root());
        assert_eq!(block.header.merkle_root, Hash::ZERO);
    }

    #[test]
    fn test_total_gas_limit() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx1 = Transaction::new(0, from, Some(to), 100, vec![], 21000, 1).signed(&keypair);
        let tx2 = Transaction::new(1, from, Some(to), 100, vec![], 30000, 1).signed(&keypair);

        let block = Block::new(1, Hash::ZERO, vec![tx1, tx2], Hash::ZERO, from);

        assert_eq!(block.total_gas_limit(), 51000);
    }
}
