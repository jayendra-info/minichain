//! Main blockchain orchestration.
//!
//! This module brings together all components: consensus, storage, mempool, and execution.

use crate::executor::{BlockExecutionResult, Executor};
use crate::mempool::Mempool;
use minichain_consensus::{Authority, BlockProposer, BlockValidator, PoAConfig, TransactionValidator};
use minichain_core::{Address, Block, Hash, Transaction};
use minichain_storage::{ChainStore, StateManager, Storage};
use thiserror::Error;

/// Errors that can occur during blockchain operations.
#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("storage error: {0}")]
    Storage(#[from] minichain_storage::StorageError),

    #[error("consensus error: {0}")]
    Consensus(#[from] minichain_consensus::ConsensusError),

    #[error("validation error: {0}")]
    Validation(#[from] minichain_consensus::ValidationError),

    #[error("execution error: {0}")]
    Execution(#[from] crate::executor::ExecutionError),

    #[error("mempool error: {0}")]
    Mempool(#[from] crate::mempool::MempoolError),

    #[error("genesis block not found")]
    MissingGenesis,

    #[error("block not found: {0:?}")]
    BlockNotFound(Hash),

    #[error("invalid chain state")]
    InvalidChainState,
}

pub type Result<T> = std::result::Result<T, BlockchainError>;

/// Blockchain configuration.
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    /// PoA consensus configuration.
    pub consensus: PoAConfig,
    /// Maximum transactions per block.
    pub max_block_size: usize,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            consensus: PoAConfig::default(),
            max_block_size: 1000,
        }
    }
}

/// Main blockchain struct that orchestrates all components.
pub struct Blockchain<'a> {
    /// Storage backend.
    #[allow(dead_code)]
    storage: &'a Storage,
    /// Chain store for blocks.
    chain: ChainStore<'a>,
    /// State manager for accounts.
    state: StateManager<'a>,
    /// Transaction mempool.
    mempool: Mempool,
    /// Consensus authority.
    authority: Authority,
    /// Configuration.
    config: BlockchainConfig,
}

impl<'a> Blockchain<'a> {
    /// Create a new blockchain with the given storage and configuration.
    pub fn new(storage: &'a Storage, config: BlockchainConfig) -> Self {
        let chain = ChainStore::new(storage);
        let state = StateManager::new(storage);
        let mempool = Mempool::new();
        let authority = Authority::new(config.consensus.clone());

        Self {
            storage,
            chain,
            state,
            mempool,
            authority,
            config,
        }
    }

    /// Initialize the blockchain with a genesis block.
    pub fn init_genesis(&self, genesis: &Block) -> Result<()> {
        self.chain.init_genesis(genesis)?;
        Ok(())
    }

    /// Get the current chain height.
    pub fn height(&self) -> Result<u64> {
        Ok(self.chain.get_height()?)
    }

    /// Get the latest block.
    pub fn get_latest_block(&self) -> Result<Block> {
        let height = self.height()?;
        self.chain
            .get_block_by_height(height)?
            .ok_or(BlockchainError::InvalidChainState)
    }

    /// Get a block by hash.
    pub fn get_block(&self, hash: &Hash) -> Result<Option<Block>> {
        Ok(self.chain.get_block_by_hash(hash)?)
    }

    /// Get a block by height.
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        Ok(self.chain.get_block_by_height(height)?)
    }

    /// Register a public key for an authority.
    pub fn register_authority(
        &mut self,
        address: Address,
        public_key: minichain_core::PublicKey,
    ) {
        self.authority.register_public_key(address, public_key);
    }

    /// Submit a transaction to the mempool.
    pub fn submit_transaction(&mut self, tx: Transaction) -> Result<()> {
        // Basic validation
        TransactionValidator::validate_transaction(&tx)?;

        // Add to mempool
        self.mempool.add(tx)?;

        Ok(())
    }

    /// Get pending transactions from the mempool.
    pub fn get_pending_transactions(&self, limit: usize) -> Vec<Transaction> {
        self.mempool.get_pending(limit)
    }

    /// Propose a new block (for authorities).
    pub fn propose_block(&mut self, proposer: &BlockProposer) -> Result<Block> {
        // Get parent block
        let parent = self.get_latest_block()?;
        let parent_hash = parent.hash();
        let new_height = parent.header.height + 1;

        // Select transactions from mempool
        let transactions = self
            .mempool
            .get_pending(self.config.max_block_size);

        // Create block (proposer checks if it's their turn)
        let block = proposer.propose_block(
            new_height,
            parent_hash,
            transactions,
            parent.header.state_root, // We'll update after execution
        )?;

        Ok(block)
    }

    /// Validate and execute a block.
    ///
    /// This performs:
    /// 1. Consensus validation (authority, signature, timestamp)
    /// 2. Block structure validation
    /// 3. Transaction execution
    /// 4. State root update
    pub fn validate_and_execute_block(&self, block: &Block) -> Result<BlockExecutionResult> {
        // Get parent block
        let parent = if block.is_genesis() {
            // Genesis has no parent
            Block::genesis(block.header.author)
        } else {
            self.chain
                .get_block_by_hash(&block.header.prev_hash)?
                .ok_or_else(|| BlockchainError::BlockNotFound(block.header.prev_hash))?
        };

        // Consensus validation
        if !block.is_genesis() {
            self.authority
                .verify_block(block, parent.header.timestamp)?;
        }

        // Block structure validation
        BlockValidator::validate_full(block, parent.hash(), parent.header.height)?;

        // Execute transactions
        let executor = Executor::new(&self.state);
        let result = executor.execute_block(block)?;

        Ok(result)
    }

    /// Import a block into the chain.
    ///
    /// This validates, executes, and stores the block.
    pub fn import_block(&mut self, block: Block) -> Result<BlockExecutionResult> {
        // Validate and execute
        let result = self.validate_and_execute_block(&block)?;

        let block_hash = block.hash();
        let block_height = block.header.height;

        // Store block
        self.chain.put_block(&block)?;

        // Update chain head
        self.chain.set_head(&block_hash, block_height)?;

        // Remove included transactions from mempool
        let tx_hashes: Vec<_> = block.transactions.iter().map(|tx| tx.hash()).collect();
        self.mempool.remove_batch(&tx_hashes);

        Ok(result)
    }

    /// Get blockchain statistics.
    pub fn stats(&self) -> Result<BlockchainStats> {
        let height = self.height()?;
        let latest_block = self.get_latest_block()?;
        let mempool_stats = self.mempool.stats();

        Ok(BlockchainStats {
            height,
            latest_block_hash: latest_block.hash(),
            latest_timestamp: latest_block.header.timestamp,
            pending_transactions: mempool_stats.total_transactions,
            authority_count: self.authority.config().authority_count(),
        })
    }
}

/// Blockchain statistics.
#[derive(Debug, Clone)]
pub struct BlockchainStats {
    /// Current chain height.
    pub height: u64,
    /// Hash of the latest block.
    pub latest_block_hash: Hash,
    /// Timestamp of the latest block.
    pub latest_timestamp: u64,
    /// Number of pending transactions.
    pub pending_transactions: usize,
    /// Number of authorities.
    pub authority_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use minichain_core::Keypair;

    fn setup_blockchain() -> (&'static Storage, Blockchain<'static>, Keypair) {
        let storage: &'static Storage = Box::leak(Box::new(Storage::open_temporary().unwrap()));
        let keypair = Keypair::generate();
        let addr = keypair.address();

        let config = BlockchainConfig {
            consensus: PoAConfig::new(vec![addr], 5),
            max_block_size: 100,
        };

        let mut blockchain = Blockchain::new(storage, config);
        blockchain.register_authority(addr, keypair.public_key.clone());

        // Initialize genesis with timestamp in the past to avoid timing issues
        let mut genesis = Block::genesis(addr);
        genesis.header.timestamp -= 10; // Set genesis timestamp 10 seconds in the past
        genesis.sign(&keypair);
        blockchain.init_genesis(&genesis).unwrap();

        (storage, blockchain, keypair)
    }

    #[test]
    fn test_blockchain_init() {
        let (_storage, blockchain, _) = setup_blockchain();

        assert_eq!(blockchain.height().unwrap(), 0);
        let genesis = blockchain.get_latest_block().unwrap();
        assert!(genesis.is_genesis());
    }

    #[test]
    fn test_submit_transaction() {
        let (_storage, mut blockchain, keypair) = setup_blockchain();

        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);
        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        assert!(blockchain.submit_transaction(tx).is_ok());
        assert_eq!(blockchain.mempool.len(), 1);
    }

    #[test]
    fn test_propose_block() {
        let (_storage, mut blockchain, keypair) = setup_blockchain();

        // Add some transactions
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);
        let tx1 = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        let tx2 = Transaction::transfer(from, to, 2000, 1, 1).signed(&keypair);

        blockchain.submit_transaction(tx1).unwrap();
        blockchain.submit_transaction(tx2).unwrap();

        // Create proposer
        let config = PoAConfig::new(vec![from], 5);
        let proposer = BlockProposer::new(keypair, config);

        // Propose block
        let block = blockchain.propose_block(&proposer).unwrap();

        assert_eq!(block.header.height, 1);
        assert_eq!(block.transactions.len(), 2);
    }

    #[test]
    fn test_import_block() {
        let (_storage, mut blockchain, keypair) = setup_blockchain();

        let from = keypair.address();

        // Fund the account
        blockchain
            .state
            .put_account(&from, &minichain_core::Account::new_user(1_000_000))
            .unwrap();

        // Add transactions
        let to = Address::from_bytes([2u8; 20]);
        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);
        blockchain.submit_transaction(tx.clone()).unwrap();

        // Create proposer
        let config = PoAConfig::new(vec![from], 5);
        let proposer = BlockProposer::new(keypair, config);

        // Propose and import block (block is already signed by propose_block)
        let block = blockchain.propose_block(&proposer).unwrap();

        let result = blockchain.import_block(block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert_eq!(blockchain.height().unwrap(), 1);
        assert_eq!(blockchain.mempool.len(), 0); // Transaction removed from mempool
    }

    #[test]
    fn test_blockchain_stats() {
        let (_storage, blockchain, _) = setup_blockchain();

        let stats = blockchain.stats().unwrap();
        assert_eq!(stats.height, 0);
        assert_eq!(stats.pending_transactions, 0);
        assert_eq!(stats.authority_count, 1);
    }
}
