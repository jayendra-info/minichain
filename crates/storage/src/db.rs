//! sled database wrapper with serialization helpers.

use minichain_core::Address;
use minichain_core::Hash;
use sled::Db;
use std::path::Path;
use thiserror::Error;

/// Storage errors.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sled::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Insufficient balance: address {address}, required {required}, available {available}")]
    InsufficientBalance {
        address: Address,
        required: u64,
        available: u64,
    },

    #[error("Invalid genesis: {0}")]
    InvalidGenesis(String),

    #[error("Invalid storage value")]
    InvalidStorageValue,
}

/// Result type for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

/// Wrapper around sled database with serialization helpers.
pub struct Storage {
    db: Db,
}

impl Storage {
    /// Open a database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Open an in-memory database (for testing).
    pub fn open_temporary() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;
        Ok(Self { db })
    }

    /// Store a serializable value.
    pub fn put<K, V>(&self, key: K, value: &V) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let encoded = bincode::serialize(value)?;
        self.db.insert(key, encoded)?;
        Ok(())
    }

    /// Retrieve and deserialize a value.
    pub fn get<K, V>(&self, key: K) -> Result<Option<V>>
    where
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        match self.db.get(key)? {
            Some(bytes) => {
                let value = bincode::deserialize(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Retrieve a value, returning error if not found.
    pub fn get_or_err<K, V>(&self, key: K) -> Result<V>
    where
        K: AsRef<[u8]> + std::fmt::Debug + Clone,
        V: serde::de::DeserializeOwned,
    {
        self.get(key.clone())?
            .ok_or_else(|| StorageError::NotFound(format!("{:?}", key)))
    }

    /// Delete a key.
    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<()> {
        self.db.remove(key)?;
        Ok(())
    }

    /// Check if a key exists.
    pub fn contains<K: AsRef<[u8]>>(&self, key: K) -> Result<bool> {
        Ok(self.db.contains_key(key)?)
    }

    /// Get the underlying sled database (for advanced operations like scan_prefix).
    pub fn inner(&self) -> &Db {
        &self.db
    }

    /// Apply multiple operations atomically.
    ///
    /// Note: Atomicity is provided by sled's `apply_batch`, not by Rust's `FnOnce`.
    /// The batch collects operations in memory, then `apply_batch` writes them
    /// atomically using sled's write-ahead log (WAL).
    pub fn batch(&self, operations: Vec<BatchOp>) -> Result<()> {
        let mut batch = sled::Batch::default();
        for op in operations {
            match op {
                BatchOp::Insert { key, value } => batch.insert(key, value),
                BatchOp::Remove { key } => batch.remove(key),
            }
        }
        self.db.apply_batch(batch)?;
        Ok(())
    }

    /// Flush all pending writes to disk.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    // =========================================================================
    // Key Construction Helpers
    // =========================================================================

    /// Create a prefixed key for accounts.
    /// Format: "account:" + address_bytes
    pub fn account_key(address: &Address) -> Vec<u8> {
        let mut key = b"account:".to_vec();
        key.extend_from_slice(&address.0);
        key
    }

    /// Create a prefixed key for blocks by height.
    /// Format: "block:height:{height}"
    pub fn block_height_key(height: u64) -> Vec<u8> {
        format!("block:height:{}", height).into_bytes()
    }

    /// Create a prefixed key for blocks by hash.
    /// Format: "block:hash:" + hash_bytes
    pub fn block_hash_key(hash: &Hash) -> Vec<u8> {
        let mut key = b"block:hash:".to_vec();
        key.extend_from_slice(&hash.0);
        key
    }

    /// Create a prefixed key for contract storage.
    /// Format: "storage:" + contract_address + ":" + slot
    pub fn contract_storage_key(contract: &Address, slot: &[u8]) -> Vec<u8> {
        let mut key = b"storage:".to_vec();
        key.extend_from_slice(&contract.0);
        key.push(b':');
        key.extend_from_slice(slot);
        key
    }

    /// Create a prefixed key for contract code.
    /// Format: "code:{code_hash_hex}"
    pub fn code_key(code_hash: &Hash) -> String {
        format!("code:{}", code_hash.to_hex())
    }
}

/// Batch operation for atomic updates.
pub enum BatchOp {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Remove { key: Vec<u8> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_temporary() {
        let storage = Storage::open_temporary().unwrap();
        assert!(storage.db.is_empty());
    }

    #[test]
    fn test_put_get() {
        let storage = Storage::open_temporary().unwrap();

        // Store a value
        storage.put("key1", &42u64).unwrap();

        // Retrieve it
        let value: Option<u64> = storage.get("key1").unwrap();
        assert_eq!(value, Some(42));

        // Non-existent key returns None
        let missing: Option<u64> = storage.get("missing").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_get_or_err() {
        let storage = Storage::open_temporary().unwrap();

        storage.put("exists", &100u64).unwrap();

        // Existing key works
        let value: u64 = storage.get_or_err("exists").unwrap();
        assert_eq!(value, 100);

        // Missing key returns error
        let result: Result<u64> = storage.get_or_err("missing");
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn test_delete() {
        let storage = Storage::open_temporary().unwrap();

        storage.put("key", &"value").unwrap();
        assert!(storage.contains("key").unwrap());

        storage.delete("key").unwrap();
        assert!(!storage.contains("key").unwrap());
    }

    #[test]
    fn test_batch_operations() {
        let storage = Storage::open_temporary().unwrap();

        // Batch insert
        let ops = vec![
            BatchOp::Insert {
                key: b"a".to_vec(),
                value: bincode::serialize(&1u64).unwrap(),
            },
            BatchOp::Insert {
                key: b"b".to_vec(),
                value: bincode::serialize(&2u64).unwrap(),
            },
        ];
        storage.batch(ops).unwrap();

        let a: u64 = storage.get("a").unwrap().unwrap();
        let b: u64 = storage.get("b").unwrap().unwrap();
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }

    #[test]
    fn test_key_construction() {
        let address = Address([0xAA; 20]);
        let hash = Hash([0xBB; 32]);

        let account_key = Storage::account_key(&address);
        assert!(account_key.starts_with(b"account:"));

        let height_key = Storage::block_height_key(42);
        assert_eq!(height_key, b"block:height:42");

        let hash_key = Storage::block_hash_key(&hash);
        assert!(hash_key.starts_with(b"block:hash:"));

        let storage_key = Storage::contract_storage_key(&address, &[0, 0, 0, 1]);
        assert!(storage_key.starts_with(b"storage:"));
    }
}
