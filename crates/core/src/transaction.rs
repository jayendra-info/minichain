//! Transaction types and signing.

use crate::crypto::{Address, Keypair, PublicKey, Signature};
use crate::hash::{hash, Hash};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during transaction operations.
#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("missing signature")]
    MissingSignature,
}

/// A transaction on the blockchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender's nonce (sequence number).
    pub nonce: u64,
    /// Sender's address.
    pub from: Address,
    /// Recipient's address (None for contract deployment).
    pub to: Option<Address>,
    /// Value to transfer.
    pub value: u64,
    /// Transaction data (calldata or contract bytecode).
    pub data: Vec<u8>,
    /// Maximum gas to use.
    pub gas_limit: u64,
    /// Price per unit of gas.
    pub gas_price: u64,
    /// Transaction signature.
    pub signature: Signature,
}

/// Unsigned transaction data (for hashing and signing).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsignedTransaction {
    nonce: u64,
    from: Address,
    to: Option<Address>,
    value: u64,
    data: Vec<u8>,
    gas_limit: u64,
    gas_price: u64,
}

impl Transaction {
    /// Create a new unsigned transaction.
    pub fn new(
        nonce: u64,
        from: Address,
        to: Option<Address>,
        value: u64,
        data: Vec<u8>,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        Self {
            nonce,
            from,
            to,
            value,
            data,
            gas_limit,
            gas_price,
            signature: Signature::default(),
        }
    }

    /// Create a value transfer transaction.
    pub fn transfer(from: Address, to: Address, value: u64, nonce: u64, gas_price: u64) -> Self {
        Self::new(
            nonce,
            from,
            Some(to),
            value,
            Vec::new(),
            21_000, // Base gas for transfer
            gas_price,
        )
    }

    /// Create a contract deployment transaction.
    pub fn deploy(from: Address, bytecode: Vec<u8>, nonce: u64, gas_limit: u64, gas_price: u64) -> Self {
        Self::new(nonce, from, None, 0, bytecode, gas_limit, gas_price)
    }

    /// Create a contract call transaction.
    pub fn call(
        from: Address,
        to: Address,
        data: Vec<u8>,
        value: u64,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        Self::new(nonce, from, Some(to), value, data, gas_limit, gas_price)
    }

    /// Get the hash of the unsigned transaction (for signing).
    pub fn signing_hash(&self) -> Hash {
        let unsigned = UnsignedTransaction {
            nonce: self.nonce,
            from: self.from,
            to: self.to,
            value: self.value,
            data: self.data.clone(),
            gas_limit: self.gas_limit,
            gas_price: self.gas_price,
        };
        let encoded = bincode::serialize(&unsigned).expect("serialization should not fail");
        hash(&encoded)
    }

    /// Get the full transaction hash (including signature).
    pub fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).expect("serialization should not fail");
        hash(&encoded)
    }

    /// Sign the transaction with the given keypair.
    pub fn sign(&mut self, keypair: &Keypair) {
        let hash = self.signing_hash();
        self.signature = keypair.sign_hash(&hash);
    }

    /// Create a signed transaction.
    pub fn signed(mut self, keypair: &Keypair) -> Self {
        self.sign(keypair);
        self
    }

    /// Verify the transaction signature.
    pub fn verify(&self, public_key: &PublicKey) -> Result<(), TransactionError> {
        let hash = self.signing_hash();
        public_key
            .verify(hash.as_bytes(), &self.signature)
            .map_err(|_| TransactionError::VerificationFailed)
    }

    /// Check if this is a contract deployment transaction.
    pub fn is_deploy(&self) -> bool {
        self.to.is_none()
    }

    /// Check if this is a simple value transfer.
    pub fn is_transfer(&self) -> bool {
        self.to.is_some() && self.data.is_empty()
    }

    /// Check if this is a contract call.
    pub fn is_call(&self) -> bool {
        self.to.is_some() && !self.data.is_empty()
    }

    /// Calculate the maximum cost of this transaction.
    pub fn max_cost(&self) -> u64 {
        self.value.saturating_add(self.gas_limit.saturating_mul(self.gas_price))
    }

    /// Calculate the contract address for a deployment transaction.
    /// Returns None if this is not a deployment transaction.
    pub fn contract_address(&self) -> Option<Address> {
        if !self.is_deploy() {
            return None;
        }
        // Contract address = first 20 bytes of hash(sender || nonce)
        let mut data = Vec::new();
        data.extend_from_slice(&self.from.0);
        data.extend_from_slice(&self.nonce.to_le_bytes());
        let h = hash(&data);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&h.0[..20]);
        Some(Address(addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_transaction() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let tx = Transaction::transfer(from, to, 1000, 0, 1);

        assert!(tx.is_transfer());
        assert!(!tx.is_deploy());
        assert!(!tx.is_call());
        assert_eq!(tx.value, 1000);
        assert_eq!(tx.gas_limit, 21_000);
    }

    #[test]
    fn test_deploy_transaction() {
        let from = Address::from_bytes([1u8; 20]);
        let bytecode = vec![0x01, 0x02, 0x03];
        let tx = Transaction::deploy(from, bytecode.clone(), 0, 100_000, 1);

        assert!(tx.is_deploy());
        assert!(!tx.is_transfer());
        assert!(!tx.is_call());
        assert_eq!(tx.data, bytecode);
        assert!(tx.contract_address().is_some());
    }

    #[test]
    fn test_call_transaction() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let calldata = vec![0xab, 0xcd];
        let tx = Transaction::call(from, to, calldata.clone(), 0, 0, 50_000, 1);

        assert!(tx.is_call());
        assert!(!tx.is_transfer());
        assert!(!tx.is_deploy());
        assert_eq!(tx.data, calldata);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        assert!(tx.verify(&keypair.public_key).is_ok());
    }

    #[test]
    fn test_wrong_key_verification_fails() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let from = keypair1.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair1);

        assert!(tx.verify(&keypair2.public_key).is_err());
    }

    #[test]
    fn test_transaction_hash_deterministic() {
        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        let h1 = tx.hash();
        let h2 = tx.hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_max_cost() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        let tx = Transaction::new(0, from, Some(to), 1000, vec![], 50_000, 2);

        assert_eq!(tx.max_cost(), 1000 + 50_000 * 2);
    }

    #[test]
    fn test_contract_address_deterministic() {
        let from = Address::from_bytes([1u8; 20]);
        let bytecode = vec![0x01, 0x02, 0x03];

        let tx1 = Transaction::deploy(from, bytecode.clone(), 0, 100_000, 1);
        let tx2 = Transaction::deploy(from, bytecode.clone(), 0, 100_000, 1);
        let tx3 = Transaction::deploy(from, bytecode, 1, 100_000, 1); // Different nonce

        assert_eq!(tx1.contract_address(), tx2.contract_address());
        assert_ne!(tx1.contract_address(), tx3.contract_address());
    }
}
