//! Account state representation.

use crate::hash::Hash;
use serde::{Deserialize, Serialize};

/// An account in the blockchain state.
///
/// Accounts can be either:
/// - Externally Owned Accounts (EOA): User accounts with no code
/// - Contract Accounts: Accounts with associated bytecode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Transaction count / sequence number.
    pub nonce: u64,
    /// Account balance in the native token.
    pub balance: u64,
    /// Hash of the contract bytecode (None for EOAs).
    pub code_hash: Option<Hash>,
    /// Root hash of the account's storage trie.
    pub storage_root: Hash,
}

impl Account {
    /// Create a new user account (EOA) with the given balance.
    pub fn new_user(balance: u64) -> Self {
        Self {
            nonce: 0,
            balance,
            code_hash: None,
            storage_root: Hash::ZERO,
        }
    }

    /// Create a new contract account with the given code hash.
    pub fn new_contract(code_hash: Hash) -> Self {
        Self {
            nonce: 0,
            balance: 0,
            code_hash: Some(code_hash),
            storage_root: Hash::ZERO,
        }
    }

    /// Check if this is a contract account.
    pub fn is_contract(&self) -> bool {
        self.code_hash.is_some()
    }

    /// Check if this is an externally owned account (EOA).
    pub fn is_eoa(&self) -> bool {
        self.code_hash.is_none()
    }

    /// Increment the nonce.
    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.saturating_add(1);
    }

    /// Add balance to the account.
    pub fn credit(&mut self, amount: u64) {
        self.balance = self.balance.saturating_add(amount);
    }

    /// Subtract balance from the account.
    /// Returns true if successful, false if insufficient balance.
    pub fn debit(&mut self, amount: u64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }

    /// Check if the account has sufficient balance.
    pub fn has_balance(&self, amount: u64) -> bool {
        self.balance >= amount
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new_user(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash;

    #[test]
    fn test_new_user_account() {
        let account = Account::new_user(1000);
        assert_eq!(account.nonce, 0);
        assert_eq!(account.balance, 1000);
        assert!(account.is_eoa());
        assert!(!account.is_contract());
    }

    #[test]
    fn test_new_contract_account() {
        let code_hash = hash(b"contract bytecode");
        let account = Account::new_contract(code_hash);
        assert_eq!(account.nonce, 0);
        assert_eq!(account.balance, 0);
        assert!(account.is_contract());
        assert!(!account.is_eoa());
        assert_eq!(account.code_hash, Some(code_hash));
    }

    #[test]
    fn test_nonce_increment() {
        let mut account = Account::new_user(0);
        assert_eq!(account.nonce, 0);
        account.increment_nonce();
        assert_eq!(account.nonce, 1);
        account.increment_nonce();
        assert_eq!(account.nonce, 2);
    }

    #[test]
    fn test_credit_and_debit() {
        let mut account = Account::new_user(100);

        account.credit(50);
        assert_eq!(account.balance, 150);

        assert!(account.debit(100));
        assert_eq!(account.balance, 50);

        assert!(!account.debit(100)); // Insufficient balance
        assert_eq!(account.balance, 50); // Balance unchanged
    }

    #[test]
    fn test_has_balance() {
        let account = Account::new_user(100);
        assert!(account.has_balance(50));
        assert!(account.has_balance(100));
        assert!(!account.has_balance(101));
    }

    #[test]
    fn test_default_account() {
        let account = Account::default();
        assert_eq!(account.nonce, 0);
        assert_eq!(account.balance, 0);
        assert!(account.is_eoa());
    }
}
