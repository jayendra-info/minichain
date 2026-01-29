//! Proof of Authority (PoA) consensus implementation.
//!
//! PoA is a consensus mechanism where a fixed set of pre-approved authorities
//! (validators) take turns producing blocks. It's efficient and deterministic,
//! making it ideal for private/consortium blockchains and educational purposes.

use minichain_core::{Address, Block, BlockHeader, Keypair, PublicKey};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during consensus operations.
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("unauthorized authority: {0:?}")]
    UnauthorizedAuthority(Address),

    #[error("invalid block signature")]
    InvalidSignature,

    #[error("not authority's turn (expected {expected:?}, got {got:?})")]
    NotTurn { expected: Address, got: Address },

    #[error("block timestamp is too far in the future")]
    TimestampTooFuture,

    #[error("block timestamp is earlier than parent")]
    TimestampTooEarly,

    #[error("no authorities configured")]
    NoAuthorities,
}

pub type Result<T> = std::result::Result<T, ConsensusError>;

/// Proof of Authority configuration.
#[derive(Debug, Clone)]
pub struct PoAConfig {
    /// List of authority addresses that can produce blocks.
    pub authorities: Vec<Address>,
    /// Block time target in seconds (e.g., 5 seconds per block).
    pub block_time: u64,
    /// Maximum allowed clock drift in seconds (e.g., 30 seconds).
    pub max_clock_drift: u64,
}

impl Default for PoAConfig {
    fn default() -> Self {
        Self {
            authorities: Vec::new(),
            block_time: 5,
            max_clock_drift: 30,
        }
    }
}

impl PoAConfig {
    /// Create a new PoA configuration with the given authorities.
    pub fn new(authorities: Vec<Address>, block_time: u64) -> Self {
        Self {
            authorities,
            block_time,
            max_clock_drift: 30,
        }
    }

    /// Check if an address is an authority.
    pub fn is_authority(&self, address: &Address) -> bool {
        self.authorities.contains(address)
    }

    /// Get the number of authorities.
    pub fn authority_count(&self) -> usize {
        self.authorities.len()
    }

    /// Calculate which authority should produce a block at a given height.
    /// Uses round-robin selection: height % authority_count.
    pub fn authority_at_height(&self, height: u64) -> Result<Address> {
        if self.authorities.is_empty() {
            return Err(ConsensusError::NoAuthorities);
        }
        let index = (height as usize) % self.authorities.len();
        Ok(self.authorities[index])
    }
}

/// Authority management for PoA consensus.
pub struct Authority {
    /// PoA configuration.
    config: PoAConfig,
    /// Map from address to public key for signature verification.
    public_keys: HashMap<Address, PublicKey>,
}

impl Authority {
    /// Create a new Authority with the given configuration.
    pub fn new(config: PoAConfig) -> Self {
        Self {
            config,
            public_keys: HashMap::new(),
        }
    }

    /// Register a public key for an authority address.
    pub fn register_public_key(&mut self, address: Address, public_key: PublicKey) {
        self.public_keys.insert(address, public_key);
    }

    /// Get the public key for an authority address.
    pub fn get_public_key(&self, address: &Address) -> Option<&PublicKey> {
        self.public_keys.get(address)
    }

    /// Get the configuration.
    pub fn config(&self) -> &PoAConfig {
        &self.config
    }

    /// Check if an address is an authority.
    pub fn is_authority(&self, address: &Address) -> bool {
        self.config.is_authority(address)
    }

    /// Verify that a block was produced by the correct authority.
    pub fn verify_block_authority(&self, block: &Block) -> Result<()> {
        let author = &block.header.author;

        // Check if author is an authority
        if !self.is_authority(author) {
            return Err(ConsensusError::UnauthorizedAuthority(*author));
        }

        // Check if it's this authority's turn
        let expected = self.config.authority_at_height(block.header.height)?;
        if expected != *author {
            return Err(ConsensusError::NotTurn {
                expected,
                got: *author,
            });
        }

        Ok(())
    }

    /// Verify the block signature.
    pub fn verify_block_signature(&self, block: &Block) -> Result<()> {
        let author = &block.header.author;
        let public_key = self
            .get_public_key(author)
            .ok_or(ConsensusError::UnauthorizedAuthority(*author))?;

        if !block.verify_signature(public_key) {
            return Err(ConsensusError::InvalidSignature);
        }

        Ok(())
    }

    /// Verify the block timestamp is valid.
    pub fn verify_block_timestamp(
        &self,
        block: &Block,
        parent_timestamp: u64,
        now: u64,
    ) -> Result<()> {
        // Block timestamp must be after parent
        if block.header.timestamp <= parent_timestamp {
            return Err(ConsensusError::TimestampTooEarly);
        }

        // Block timestamp cannot be too far in the future
        if block.header.timestamp > now + self.config.max_clock_drift {
            return Err(ConsensusError::TimestampTooFuture);
        }

        Ok(())
    }

    /// Verify all consensus rules for a block.
    pub fn verify_block(&self, block: &Block, parent_timestamp: u64) -> Result<()> {
        let now = BlockHeader::current_timestamp();

        // Verify authority
        self.verify_block_authority(block)?;

        // Verify signature
        self.verify_block_signature(block)?;

        // Verify timestamp
        self.verify_block_timestamp(block, parent_timestamp, now)?;

        Ok(())
    }
}

/// Block proposer for authorities.
pub struct BlockProposer {
    /// Authority keypair for signing blocks.
    keypair: Keypair,
    /// Authority configuration.
    config: PoAConfig,
}

impl BlockProposer {
    /// Create a new block proposer with the given keypair and config.
    pub fn new(keypair: Keypair, config: PoAConfig) -> Self {
        Self { keypair, config }
    }

    /// Get the proposer's address.
    pub fn address(&self) -> Address {
        self.keypair.address()
    }

    /// Check if this proposer can produce a block at the given height.
    pub fn can_propose_at_height(&self, height: u64) -> Result<bool> {
        let expected = self.config.authority_at_height(height)?;
        Ok(expected == self.address())
    }

    /// Propose a new block (creates and signs it).
    pub fn propose_block(
        &self,
        height: u64,
        prev_hash: minichain_core::Hash,
        transactions: Vec<minichain_core::Transaction>,
        state_root: minichain_core::Hash,
    ) -> Result<Block> {
        // Verify it's our turn
        if !self.can_propose_at_height(height)? {
            let expected = self.config.authority_at_height(height)?;
            return Err(ConsensusError::NotTurn {
                expected,
                got: self.address(),
            });
        }

        // Create and sign the block
        let block = Block::new(height, prev_hash, transactions, state_root, self.address())
            .signed(&self.keypair);

        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poa_config_authority_at_height() {
        let addr1 = Address::from_bytes([1u8; 20]);
        let addr2 = Address::from_bytes([2u8; 20]);
        let addr3 = Address::from_bytes([3u8; 20]);

        let config = PoAConfig::new(vec![addr1, addr2, addr3], 5);

        // Round-robin selection
        assert_eq!(config.authority_at_height(0).unwrap(), addr1);
        assert_eq!(config.authority_at_height(1).unwrap(), addr2);
        assert_eq!(config.authority_at_height(2).unwrap(), addr3);
        assert_eq!(config.authority_at_height(3).unwrap(), addr1); // Wraps around
        assert_eq!(config.authority_at_height(4).unwrap(), addr2);
    }

    #[test]
    fn test_is_authority() {
        let addr1 = Address::from_bytes([1u8; 20]);
        let addr2 = Address::from_bytes([2u8; 20]);
        let addr3 = Address::from_bytes([3u8; 20]);

        let config = PoAConfig::new(vec![addr1, addr2], 5);

        assert!(config.is_authority(&addr1));
        assert!(config.is_authority(&addr2));
        assert!(!config.is_authority(&addr3));
    }

    #[test]
    fn test_authority_verification() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let addr1 = keypair1.address();
        let addr2 = keypair2.address();

        let config = PoAConfig::new(vec![addr1, addr2], 5);
        let mut authority = Authority::new(config);
        authority.register_public_key(addr1, keypair1.public_key.clone());
        authority.register_public_key(addr2, keypair2.public_key.clone());

        // Height 0 should be produced by authority 0 (addr1)
        let block = Block::genesis(addr1).signed(&keypair1);
        assert!(authority.verify_block_authority(&block).is_ok());
        assert!(authority.verify_block_signature(&block).is_ok());

        // Height 1 should be produced by authority 1 (addr2)
        let block = Block::new(
            1,
            minichain_core::Hash::ZERO,
            vec![],
            minichain_core::Hash::ZERO,
            addr2,
        )
        .signed(&keypair2);
        assert!(authority.verify_block_authority(&block).is_ok());
        assert!(authority.verify_block_signature(&block).is_ok());
    }

    #[test]
    fn test_wrong_turn_rejected() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let addr1 = keypair1.address();
        let addr2 = keypair2.address();

        let config = PoAConfig::new(vec![addr1, addr2], 5);
        let mut authority = Authority::new(config);
        authority.register_public_key(addr1, keypair1.public_key.clone());
        authority.register_public_key(addr2, keypair2.public_key.clone());

        // Height 0 should be produced by addr1, but we use addr2
        let block = Block::genesis(addr2);
        assert!(matches!(
            authority.verify_block_authority(&block),
            Err(ConsensusError::NotTurn { .. })
        ));
    }

    #[test]
    fn test_unauthorized_authority_rejected() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let addr1 = keypair1.address();
        let addr2 = keypair2.address();

        // Only addr1 is an authority
        let config = PoAConfig::new(vec![addr1], 5);
        let authority = Authority::new(config);

        // addr2 tries to produce a block
        let block = Block::genesis(addr2);
        assert!(matches!(
            authority.verify_block_authority(&block),
            Err(ConsensusError::UnauthorizedAuthority(_))
        ));
    }

    #[test]
    fn test_block_proposer() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let addr1 = keypair1.address();
        let addr2 = keypair2.address();

        let config = PoAConfig::new(vec![addr1, addr2], 5);
        let proposer = BlockProposer::new(keypair1, config);

        // Height 0 - proposer's turn
        assert!(proposer.can_propose_at_height(0).unwrap());
        let block = proposer
            .propose_block(
                0,
                minichain_core::Hash::ZERO,
                vec![],
                minichain_core::Hash::ZERO,
            )
            .unwrap();
        assert_eq!(block.header.author, addr1);
        assert_eq!(block.header.height, 0);

        // Height 1 - not proposer's turn
        assert!(!proposer.can_propose_at_height(1).unwrap());
        let result = proposer.propose_block(
            1,
            minichain_core::Hash::ZERO,
            vec![],
            minichain_core::Hash::ZERO,
        );
        assert!(matches!(result, Err(ConsensusError::NotTurn { .. })));
    }

    #[test]
    fn test_timestamp_validation() {
        let keypair = Keypair::generate();
        let addr = keypair.address();

        let config = PoAConfig::new(vec![addr], 5);
        let mut authority = Authority::new(config);
        authority.register_public_key(addr, keypair.public_key.clone());

        let mut block = Block::genesis(addr).signed(&keypair);
        let parent_timestamp = block.header.timestamp;

        // Valid: newer timestamp
        block.header.timestamp = parent_timestamp + 10;
        let now = BlockHeader::current_timestamp();
        assert!(authority
            .verify_block_timestamp(&block, parent_timestamp, now)
            .is_ok());

        // Invalid: same timestamp as parent
        block.header.timestamp = parent_timestamp;
        assert!(matches!(
            authority.verify_block_timestamp(&block, parent_timestamp, now),
            Err(ConsensusError::TimestampTooEarly)
        ));

        // Invalid: timestamp too far in future
        block.header.timestamp = now + 1000;
        assert!(matches!(
            authority.verify_block_timestamp(&block, parent_timestamp, now),
            Err(ConsensusError::TimestampTooFuture)
        ));
    }
}
