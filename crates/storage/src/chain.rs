//! Block storage and chain state management.

use crate::db::{Result, Storage, StorageError};
use minichain_core::{Block, Hash};

/// Keys for chain metadata.
const CHAIN_HEAD_KEY: &[u8] = b"chain:head";
const CHAIN_HEIGHT_KEY: &[u8] = b"chain:height";

/// Manages block storage and chain state.
pub struct ChainStore<'a> {
    storage: &'a Storage,
}

impl<'a> ChainStore<'a> {
    /// Create a new ChainStore wrapping the given storage.
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    // =========================================================================
    // Block Storage
    // =========================================================================

    /// Store a block with both height and hash indexes.
    ///
    /// We create two index entries:
    /// - Primary: `block:hash:{hash}` → full block data (immutable)
    /// - Secondary: `block:height:{height}` → hash (pointer, can change during reorgs)
    pub fn put_block(&self, block: &Block) -> Result<()> {
        let hash = block.hash();

        // Primary storage: block data by hash
        // This is the "source of truth" — immutable once written
        let hash_key = Storage::block_hash_key(&hash);
        self.storage.put(&hash_key, block)?;

        // Secondary index: hash by height
        // This is a pointer — can be updated during reorgs
        let height_key = Storage::block_height_key(block.header.height);
        self.storage.put(&height_key, &hash)?;

        Ok(())
    }

    /// Get a block by its hash.
    pub fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        let key = Storage::block_hash_key(hash);
        self.storage.get(key)
    }

    /// Get a block by its height.
    ///
    /// This performs two lookups:
    /// 1. height → hash (secondary index)
    /// 2. hash → block (primary storage)
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let height_key = Storage::block_height_key(height);
        let hash: Option<Hash> = self.storage.get(&height_key)?;

        match hash {
            Some(h) => self.get_block_by_hash(&h),
            None => Ok(None),
        }
    }

    /// Check if a block exists by hash.
    pub fn has_block(&self, hash: &Hash) -> Result<bool> {
        let key = Storage::block_hash_key(hash);
        self.storage.contains(key)
    }

    // =========================================================================
    // Chain Head Tracking
    // =========================================================================

    /// Get the current chain head hash.
    pub fn get_head(&self) -> Result<Option<Hash>> {
        self.storage.get(CHAIN_HEAD_KEY)
    }

    /// Get the current chain height.
    /// Returns 0 if the chain is not initialized.
    pub fn get_height(&self) -> Result<u64> {
        Ok(self.storage.get::<_, u64>(CHAIN_HEIGHT_KEY)?.unwrap_or(0))
    }

    /// Update the chain head (called after adding a new block).
    pub fn set_head(&self, hash: &Hash, height: u64) -> Result<()> {
        self.storage.put(CHAIN_HEAD_KEY, hash)?;
        self.storage.put(CHAIN_HEIGHT_KEY, &height)?;
        Ok(())
    }

    /// Get the latest block.
    pub fn get_latest_block(&self) -> Result<Option<Block>> {
        match self.get_head()? {
            Some(hash) => self.get_block_by_hash(&hash),
            None => Ok(None),
        }
    }

    // =========================================================================
    // Genesis Block
    // =========================================================================

    /// Initialize the chain with a genesis block.
    ///
    /// This will fail if:
    /// - The block height is not 0
    /// - The chain is already initialized
    pub fn init_genesis(&self, genesis: &Block) -> Result<()> {
        // Verify it's actually a genesis block
        if genesis.header.height != 0 {
            return Err(StorageError::InvalidGenesis(
                "Genesis block must have height 0".into(),
            ));
        }

        // Check if we already have a genesis
        if self.is_initialized()? {
            return Err(StorageError::InvalidGenesis(
                "Chain already initialized".into(),
            ));
        }

        // Store the genesis block
        self.put_block(genesis)?;

        // Set it as the head
        self.set_head(&genesis.hash(), 0)?;

        Ok(())
    }

    /// Check if the chain is initialized (has a genesis block).
    pub fn is_initialized(&self) -> Result<bool> {
        Ok(self.get_head()?.is_some())
    }

    // =========================================================================
    // Chain Operations
    // =========================================================================

    /// Append a new block to the chain.
    ///
    /// This validates that:
    /// - The block height is exactly current_height + 1
    /// - The block's prev_hash matches the current head
    ///
    /// Note: This does NOT validate transactions or signatures.
    /// Full validation should be done before calling this.
    pub fn append_block(&self, block: &Block) -> Result<()> {
        let current_height = self.get_height()?;
        let current_head = self.get_head()?;

        // Validate height
        if block.header.height != current_height + 1 {
            return Err(StorageError::InvalidGenesis(format!(
                "Expected block height {}, got {}",
                current_height + 1,
                block.header.height
            )));
        }

        // Validate prev_hash (if chain is initialized)
        if let Some(head_hash) = current_head {
            if block.header.prev_hash != head_hash {
                return Err(StorageError::InvalidGenesis(format!(
                    "Block prev_hash {} doesn't match chain head {}",
                    block.header.prev_hash, head_hash
                )));
            }
        }

        // Store the block
        self.put_block(block)?;

        // Update chain head
        self.set_head(&block.hash(), block.header.height)?;

        Ok(())
    }

    /// Get blocks in a range [from_height, to_height].
    pub fn get_blocks_range(&self, from_height: u64, to_height: u64) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        for height in from_height..=to_height {
            if let Some(block) = self.get_block_by_height(height)? {
                blocks.push(block);
            } else {
                break; // Stop at first missing block
            }
        }
        Ok(blocks)
    }

    /// Get the last N blocks (most recent first).
    pub fn get_recent_blocks(&self, count: u64) -> Result<Vec<Block>> {
        let height = self.get_height()?;
        if height == 0 {
            return Ok(Vec::new());
        }

        let from_height = height.saturating_sub(count - 1);
        let mut blocks = self.get_blocks_range(from_height, height)?;
        blocks.reverse(); // Most recent first
        Ok(blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minichain_core::Address;

    fn setup() -> Storage {
        Storage::open_temporary().unwrap()
    }

    fn genesis_block() -> Block {
        let authority = Address([0xAA; 20]);
        Block::genesis(authority)
    }

    #[test]
    fn test_genesis_init() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        assert!(!chain.is_initialized().unwrap());

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        assert!(chain.is_initialized().unwrap());
        assert_eq!(chain.get_height().unwrap(), 0);
        assert_eq!(chain.get_head().unwrap(), Some(genesis.hash()));
    }

    #[test]
    fn test_genesis_double_init_fails() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        // Second init should fail
        let result = chain.init_genesis(&genesis);
        assert!(matches!(result, Err(StorageError::InvalidGenesis(_))));
    }

    #[test]
    fn test_non_genesis_as_genesis_fails() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let authority = Address([0xBB; 20]);
        let block = Block::new(1, Hash::ZERO, vec![], Hash::ZERO, authority);

        let result = chain.init_genesis(&block);
        assert!(matches!(result, Err(StorageError::InvalidGenesis(_))));
    }

    #[test]
    fn test_block_by_hash() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        let hash = genesis.hash();
        let retrieved = chain.get_block_by_hash(&hash).unwrap().unwrap();
        assert_eq!(retrieved.hash(), hash);
    }

    #[test]
    fn test_block_by_height() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        let retrieved = chain.get_block_by_height(0).unwrap().unwrap();
        assert_eq!(retrieved.hash(), genesis.hash());

        // Non-existent height returns None
        assert!(chain.get_block_by_height(1).unwrap().is_none());
    }

    #[test]
    fn test_latest_block() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        // No blocks yet
        assert!(chain.get_latest_block().unwrap().is_none());

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        let latest = chain.get_latest_block().unwrap().unwrap();
        assert_eq!(latest.hash(), genesis.hash());
    }

    #[test]
    fn test_append_block() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        // Create block 1
        let authority = Address([0xAA; 20]);
        let block1 = Block::new(1, genesis.hash(), vec![], Hash::ZERO, authority);
        chain.append_block(&block1).unwrap();

        assert_eq!(chain.get_height().unwrap(), 1);
        assert_eq!(chain.get_head().unwrap(), Some(block1.hash()));

        // Create block 2
        let block2 = Block::new(2, block1.hash(), vec![], Hash::ZERO, authority);
        chain.append_block(&block2).unwrap();

        assert_eq!(chain.get_height().unwrap(), 2);
        assert_eq!(chain.get_head().unwrap(), Some(block2.hash()));
    }

    #[test]
    fn test_append_wrong_height_fails() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        // Try to append block with wrong height
        let authority = Address([0xAA; 20]);
        let block = Block::new(5, genesis.hash(), vec![], Hash::ZERO, authority);
        let result = chain.append_block(&block);
        assert!(matches!(result, Err(StorageError::InvalidGenesis(_))));
    }

    #[test]
    fn test_append_wrong_prev_hash_fails() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        // Try to append block with wrong prev_hash
        let authority = Address([0xAA; 20]);
        let wrong_hash = Hash([0xFF; 32]);
        let block = Block::new(1, wrong_hash, vec![], Hash::ZERO, authority);
        let result = chain.append_block(&block);
        assert!(matches!(result, Err(StorageError::InvalidGenesis(_))));
    }

    #[test]
    fn test_get_blocks_range() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        let authority = Address([0xAA; 20]);
        let block1 = Block::new(1, genesis.hash(), vec![], Hash::ZERO, authority);
        chain.append_block(&block1).unwrap();

        let block2 = Block::new(2, block1.hash(), vec![], Hash::ZERO, authority);
        chain.append_block(&block2).unwrap();

        let blocks = chain.get_blocks_range(0, 2).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].header.height, 0);
        assert_eq!(blocks[1].header.height, 1);
        assert_eq!(blocks[2].header.height, 2);
    }

    #[test]
    fn test_get_recent_blocks() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        chain.init_genesis(&genesis).unwrap();

        let authority = Address([0xAA; 20]);
        let block1 = Block::new(1, genesis.hash(), vec![], Hash::ZERO, authority);
        chain.append_block(&block1).unwrap();

        let block2 = Block::new(2, block1.hash(), vec![], Hash::ZERO, authority);
        chain.append_block(&block2).unwrap();

        // Get last 2 blocks (most recent first)
        let blocks = chain.get_recent_blocks(2).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].header.height, 2);
        assert_eq!(blocks[1].header.height, 1);
    }

    #[test]
    fn test_has_block() {
        let storage = setup();
        let chain = ChainStore::new(&storage);

        let genesis = genesis_block();
        let hash = genesis.hash();

        assert!(!chain.has_block(&hash).unwrap());

        chain.init_genesis(&genesis).unwrap();

        assert!(chain.has_block(&hash).unwrap());
        assert!(!chain.has_block(&Hash::ZERO).unwrap());
    }
}
