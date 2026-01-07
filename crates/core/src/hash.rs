//! Blake3 hashing utilities for the blockchain.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A named alias for a 32-byte(u8) array, used to represent a 256-bit hash.
pub type H256 = [u8; 32];

/// A wrapper type for H256 with Display and Debug formatting.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Hash(pub H256);

impl Hash {
    /// The zero hash (all zeros).
    pub const ZERO: Self = Self([0u8; 32]);

    /// Create a new Hash from raw bytes.
    pub fn from_bytes(bytes: H256) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &H256 {
        &self.0
    }

    /// Convert to a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from a hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash(0x{})", &self.to_hex()[..8])
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", self.to_hex())
    }
}

impl From<H256> for Hash {
    fn from(bytes: H256) -> Self {
        Self(bytes)
    }
}

impl From<Hash> for H256 {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Hash arbitrary data using Blake3.
pub fn hash(data: &[u8]) -> Hash {
    Hash(blake3::hash(data).into())
}

/// Hash multiple pieces of data by concatenating them.
pub fn hash_concat(parts: &[&[u8]]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    Hash(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let data = b"hello world";
        let h1 = hash(data);
        let h2 = hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let h1 = hash(b"hello");
        let h2 = hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_hex_roundtrip() {
        let h = hash(b"test data");
        let hex_str = h.to_hex();
        let parsed = Hash::from_hex(&hex_str).unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn test_hash_display() {
        let h = hash(b"test");
        let display = format!("{}", h);
        assert!(display.starts_with("0x"));
        assert_eq!(display.len(), 66); // "0x" + 64 hex chars
    }

    #[test]
    fn test_hash_concat() {
        let h1 = hash_concat(&[b"hello", b"world"]);
        let h2 = hash(b"helloworld");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_zero_hash() {
        assert_eq!(Hash::ZERO.0, [0u8; 32]);
    }
}
