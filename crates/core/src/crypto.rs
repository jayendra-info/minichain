//! Ed25519 cryptographic primitives for signing and verification.

use crate::hash::{hash, Hash};
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// A 20-byte address derived from the public key hash.
pub type AddressBytes = [u8; 20];

/// An address on the blockchain.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Address(pub AddressBytes);

impl Address {
    /// The zero address (all zeros).
    pub const ZERO: Self = Self([0u8; 20]);

    /// Create an address from raw bytes.
    pub fn from_bytes(bytes: AddressBytes) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &AddressBytes {
        &self.0
    }

    /// Convert to a hex string (with 0x prefix).
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    /// Parse from a hex string (with or without 0x prefix).
    pub fn from_hex(s: &str) -> Result<Self, CryptoError> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|_| CryptoError::InvalidAddress)?;
        if bytes.len() != 20 {
            return Err(CryptoError::InvalidAddress);
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_hex())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A cryptographic signature.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

mod signature_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a byte slice
        serde::Serialize::serialize(bytes.as_slice(), serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("signature must be 64 bytes"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        signature_serde::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Signature(signature_serde::deserialize(deserializer)?))
    }
}

impl Signature {
    /// Create a signature from raw bytes.
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Convert to a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({}...)", &self.to_hex()[..16])
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", self.to_hex())
    }
}

/// Errors that can occur during cryptographic operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid private key")]
    InvalidPrivateKey,
    #[error("invalid address format")]
    InvalidAddress,
    #[error("signature verification failed")]
    VerificationFailed,
}

/// A public key for signature verification.
#[derive(Clone, Serialize, Deserialize)]
pub struct PublicKey(#[serde(with = "public_key_serde")] pub VerifyingKey);

mod public_key_serde {
    use ed25519_dalek::VerifyingKey;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        key.to_bytes().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <[u8; 32]>::deserialize(deserializer)?;
        VerifyingKey::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

impl PublicKey {
    /// Derive the address from this public key.
    /// Address is the first 20 bytes of the Blake3 hash of the public key.
    pub fn to_address(&self) -> Address {
        let hash = hash(self.0.as_bytes());
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash.0[..20]);
        Address(addr)
    }

    /// Get the raw bytes of the public key.
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Verify a signature against this public key.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        let sig =
            DalekSignature::from_bytes(&signature.0);
        self.0
            .verify(message, &sig)
            .map_err(|_| CryptoError::VerificationFailed)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", hex::encode(&self.0.as_bytes()[..8]))
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes() == other.0.as_bytes()
    }
}

impl Eq for PublicKey {}

/// A keypair for signing and verification.
pub struct Keypair {
    signing_key: SigningKey,
    pub public_key: PublicKey,
}

impl Keypair {
    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            public_key: PublicKey(verifying_key),
        }
    }

    /// Create a keypair from a private key (32 bytes).
    pub fn from_private_key(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        Ok(Self {
            signing_key,
            public_key: PublicKey(verifying_key),
        })
    }

    /// Get the private key bytes.
    pub fn private_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the address derived from the public key.
    pub fn address(&self) -> Address {
        self.public_key.to_address()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.signing_key.sign(message);
        Signature(sig.to_bytes())
    }

    /// Sign a hash directly.
    pub fn sign_hash(&self, hash: &Hash) -> Signature {
        self.sign(hash.as_bytes())
    }

    /// Verify a signature against our public key.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        self.public_key.verify(message, signature)
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field("address", &self.address())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = Keypair::generate();
        let addr = kp.address();
        assert_ne!(addr, Address::ZERO);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = Keypair::generate();
        let message = b"hello world";
        let sig = kp.sign(message);
        assert!(kp.verify(message, &sig).is_ok());
    }

    #[test]
    fn test_wrong_message_fails() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"hello");
        assert!(kp.verify(b"world", &sig).is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        let sig = kp1.sign(b"hello");
        assert!(kp2.verify(b"hello", &sig).is_err());
    }

    #[test]
    fn test_address_hex_roundtrip() {
        let kp = Keypair::generate();
        let addr = kp.address();
        let hex_str = addr.to_hex();
        let parsed = Address::from_hex(&hex_str).unwrap();
        assert_eq!(addr, parsed);
    }

    #[test]
    fn test_address_from_hex_no_prefix() {
        let kp = Keypair::generate();
        let addr = kp.address();
        let hex_str = hex::encode(addr.0);
        let parsed = Address::from_hex(&hex_str).unwrap();
        assert_eq!(addr, parsed);
    }

    #[test]
    fn test_keypair_from_private_key() {
        let kp1 = Keypair::generate();
        let private_key = kp1.private_key();
        let kp2 = Keypair::from_private_key(&private_key).unwrap();
        assert_eq!(kp1.address(), kp2.address());
    }

    #[test]
    fn test_deterministic_address() {
        let kp1 = Keypair::generate();
        let private_key = kp1.private_key();
        let kp2 = Keypair::from_private_key(&private_key).unwrap();
        let kp3 = Keypair::from_private_key(&private_key).unwrap();
        assert_eq!(kp2.address(), kp3.address());
    }
}
