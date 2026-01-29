//! Transaction mempool for pending transactions.
//!
//! The mempool stores valid transactions waiting to be included in a block.

use minichain_core::{Address, Hash, Transaction};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

/// Errors that can occur during mempool operations.
#[derive(Debug, Error)]
pub enum MempoolError {
    #[error("transaction already in mempool")]
    DuplicateTransaction,

    #[error("mempool is full (capacity: {0})")]
    MempoolFull(usize),

    #[error("transaction not found in mempool")]
    TransactionNotFound,
}

pub type Result<T> = std::result::Result<T, MempoolError>;

/// Configuration for the mempool.
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in the mempool.
    pub max_transactions: usize,
    /// Maximum transactions per account.
    pub max_per_account: usize,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_transactions: 10_000,
            max_per_account: 100,
        }
    }
}

/// Transaction mempool.
pub struct Mempool {
    /// Configuration.
    config: MempoolConfig,
    /// Transactions indexed by hash.
    transactions: HashMap<Hash, Transaction>,
    /// Transactions grouped by sender address.
    by_sender: HashMap<Address, VecDeque<Hash>>,
    /// Set of transaction hashes for fast lookup.
    tx_hashes: HashSet<Hash>,
}

impl Mempool {
    /// Create a new mempool with default configuration.
    pub fn new() -> Self {
        Self::with_config(MempoolConfig::default())
    }

    /// Create a new mempool with the given configuration.
    pub fn with_config(config: MempoolConfig) -> Self {
        Self {
            config,
            transactions: HashMap::new(),
            by_sender: HashMap::new(),
            tx_hashes: HashSet::new(),
        }
    }

    /// Get the number of transactions in the mempool.
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if the mempool is empty.
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Check if a transaction is in the mempool.
    pub fn contains(&self, tx_hash: &Hash) -> bool {
        self.tx_hashes.contains(tx_hash)
    }

    /// Get a transaction from the mempool.
    pub fn get(&self, tx_hash: &Hash) -> Option<&Transaction> {
        self.transactions.get(tx_hash)
    }

    /// Add a transaction to the mempool.
    pub fn add(&mut self, tx: Transaction) -> Result<()> {
        let tx_hash = tx.hash();

        // Check if transaction already exists
        if self.contains(&tx_hash) {
            return Err(MempoolError::DuplicateTransaction);
        }

        // Check global capacity
        if self.transactions.len() >= self.config.max_transactions {
            return Err(MempoolError::MempoolFull(self.config.max_transactions));
        }

        // Check per-sender capacity
        let sender_txs = self.by_sender.entry(tx.from).or_default();
        if sender_txs.len() >= self.config.max_per_account {
            return Err(MempoolError::MempoolFull(self.config.max_per_account));
        }

        // Add transaction
        sender_txs.push_back(tx_hash);
        self.tx_hashes.insert(tx_hash);
        self.transactions.insert(tx_hash, tx);

        Ok(())
    }

    /// Remove a transaction from the mempool.
    pub fn remove(&mut self, tx_hash: &Hash) -> Result<Transaction> {
        let tx = self
            .transactions
            .remove(tx_hash)
            .ok_or(MempoolError::TransactionNotFound)?;

        self.tx_hashes.remove(tx_hash);

        // Remove from sender's queue
        if let Some(sender_txs) = self.by_sender.get_mut(&tx.from) {
            sender_txs.retain(|h| h != tx_hash);
            if sender_txs.is_empty() {
                self.by_sender.remove(&tx.from);
            }
        }

        Ok(tx)
    }

    /// Remove multiple transactions from the mempool.
    pub fn remove_batch(&mut self, tx_hashes: &[Hash]) {
        for hash in tx_hashes {
            let _ = self.remove(hash);
        }
    }

    /// Get transactions from a specific sender.
    pub fn get_by_sender(&self, sender: &Address) -> Vec<Transaction> {
        self.by_sender
            .get(sender)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|h| self.transactions.get(h).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the next transaction for a sender (lowest nonce).
    pub fn get_next_for_sender(&self, sender: &Address) -> Option<Transaction> {
        self.by_sender
            .get(sender)?
            .front()
            .and_then(|hash| self.transactions.get(hash).cloned())
    }

    /// Get transactions ordered by gas price (highest first).
    ///
    /// Returns up to `limit` transactions.
    pub fn get_by_gas_price(&self, limit: usize) -> Vec<Transaction> {
        let mut txs: Vec<_> = self.transactions.values().cloned().collect();
        txs.sort_by(|a, b| b.gas_price.cmp(&a.gas_price));
        txs.truncate(limit);
        txs
    }

    /// Get pending transactions for block building.
    ///
    /// Returns up to `limit` transactions with valid nonces, ordered by gas price.
    pub fn get_pending(&self, limit: usize) -> Vec<Transaction> {
        // For simplicity, return transactions ordered by gas price
        // In a real implementation, this would:
        // 1. Check nonces against current state
        // 2. Select transactions that can be executed
        // 3. Respect account dependencies (nonce ordering)
        self.get_by_gas_price(limit)
    }

    /// Clear all transactions from the mempool.
    pub fn clear(&mut self) {
        self.transactions.clear();
        self.by_sender.clear();
        self.tx_hashes.clear();
    }

    /// Get all transactions in the mempool.
    pub fn get_all(&self) -> Vec<Transaction> {
        self.transactions.values().cloned().collect()
    }

    /// Get mempool statistics.
    pub fn stats(&self) -> MempoolStats {
        MempoolStats {
            total_transactions: self.len(),
            unique_senders: self.by_sender.len(),
            capacity: self.config.max_transactions,
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

/// Mempool statistics.
#[derive(Debug, Clone)]
pub struct MempoolStats {
    /// Total number of transactions.
    pub total_transactions: usize,
    /// Number of unique senders.
    pub unique_senders: usize,
    /// Mempool capacity.
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use minichain_core::Keypair;

    #[test]
    fn test_mempool_add_and_get() {
        let mut mempool = Mempool::new();
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let tx_hash = tx.hash();

        assert!(mempool.add(tx.clone()).is_ok());
        assert_eq!(mempool.len(), 1);
        assert!(mempool.contains(&tx_hash));
        assert_eq!(mempool.get(&tx_hash).unwrap(), &tx);
    }

    #[test]
    fn test_mempool_duplicate_rejected() {
        let mut mempool = Mempool::new();
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        assert!(mempool.add(tx.clone()).is_ok());
        assert!(matches!(
            mempool.add(tx),
            Err(MempoolError::DuplicateTransaction)
        ));
    }

    #[test]
    fn test_mempool_remove() {
        let mut mempool = Mempool::new();
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let tx_hash = tx.hash();

        mempool.add(tx.clone()).unwrap();
        assert_eq!(mempool.len(), 1);

        let removed = mempool.remove(&tx_hash).unwrap();
        assert_eq!(removed, tx);
        assert_eq!(mempool.len(), 0);
        assert!(!mempool.contains(&tx_hash));
    }

    #[test]
    fn test_mempool_by_sender() {
        let mut mempool = Mempool::new();
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx1 = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let tx2 = Transaction::transfer(from, to, 2000, 1, 1).signed(&keypair);

        mempool.add(tx1.clone()).unwrap();
        mempool.add(tx2.clone()).unwrap();

        let sender_txs = mempool.get_by_sender(&from);
        assert_eq!(sender_txs.len(), 2);
        assert!(sender_txs.contains(&tx1));
        assert!(sender_txs.contains(&tx2));
    }

    #[test]
    fn test_mempool_get_by_gas_price() {
        let mut mempool = Mempool::new();
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx1 = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let tx2 = Transaction::transfer(from, to, 2000, 1, 5).signed(&keypair);
        let tx3 = Transaction::transfer(from, to, 3000, 2, 3).signed(&keypair);

        mempool.add(tx1.clone()).unwrap();
        mempool.add(tx2.clone()).unwrap();
        mempool.add(tx3.clone()).unwrap();

        let ordered = mempool.get_by_gas_price(10);
        assert_eq!(ordered.len(), 3);
        assert_eq!(ordered[0].gas_price, 5); // tx2
        assert_eq!(ordered[1].gas_price, 3); // tx3
        assert_eq!(ordered[2].gas_price, 1); // tx1
    }

    #[test]
    fn test_mempool_capacity_limit() {
        let config = MempoolConfig {
            max_transactions: 2,
            max_per_account: 10,
        };
        let mut mempool = Mempool::with_config(config);

        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx1 = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let tx2 = Transaction::transfer(from, to, 2000, 1, 1).signed(&keypair);
        let tx3 = Transaction::transfer(from, to, 3000, 2, 1).signed(&keypair);

        assert!(mempool.add(tx1).is_ok());
        assert!(mempool.add(tx2).is_ok());
        assert!(matches!(
            mempool.add(tx3),
            Err(MempoolError::MempoolFull(2))
        ));
    }

    #[test]
    fn test_mempool_clear() {
        let mut mempool = Mempool::new();
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        mempool.add(tx).unwrap();
        assert_eq!(mempool.len(), 1);

        mempool.clear();
        assert_eq!(mempool.len(), 0);
        assert!(mempool.is_empty());
    }

    #[test]
    fn test_mempool_stats() {
        let mut mempool = Mempool::new();
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let to = Address::from_bytes([2u8; 20]);

        let tx1 = Transaction::transfer(keypair1.address(), to, 1000, 0, 1).signed(&keypair1);
        let tx2 = Transaction::transfer(keypair2.address(), to, 2000, 0, 1).signed(&keypair2);

        mempool.add(tx1).unwrap();
        mempool.add(tx2).unwrap();

        let stats = mempool.stats();
        assert_eq!(stats.total_transactions, 2);
        assert_eq!(stats.unique_senders, 2);
    }
}
