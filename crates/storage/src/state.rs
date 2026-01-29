//! World state management (accounts and contract storage).

use crate::db::{Result, Storage, StorageError};
use minichain_core::{hash, merkle_root, Account, Address, Hash};

/// Storage slot type (32 bytes, like Ethereum).
pub type StorageSlot = [u8; 32];

/// Manages the world state (all accounts and contract storage).
pub struct StateManager<'a> {
    storage: &'a Storage,
}

impl<'a> StateManager<'a> {
    /// Create a new StateManager wrapping the given storage.
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    // =========================================================================
    // Account Operations
    // =========================================================================

    /// Create or update an account.
    pub fn put_account(&self, address: &Address, account: &Account) -> Result<()> {
        let key = Storage::account_key(address);
        self.storage.put(key, account)
    }

    /// Get an account, returning default (empty) if not found.
    ///
    /// In blockchain, every address implicitly exists with zero balance and nonce.
    /// You can send coins to any address without it being "created" first.
    pub fn get_account(&self, address: &Address) -> Result<Account> {
        let key = Storage::account_key(address);
        Ok(self.storage.get::<_, Account>(key)?.unwrap_or_default())
    }

    /// Check if an account exists (has been explicitly stored).
    pub fn account_exists(&self, address: &Address) -> Result<bool> {
        let key = Storage::account_key(address);
        self.storage.contains(key)
    }

    // =========================================================================
    // Balance Operations
    // =========================================================================

    /// Get account balance.
    pub fn get_balance(&self, address: &Address) -> Result<u64> {
        Ok(self.get_account(address)?.balance)
    }

    /// Set account balance directly.
    pub fn set_balance(&self, address: &Address, balance: u64) -> Result<()> {
        let mut account = self.get_account(address)?;
        account.balance = balance;
        self.put_account(address, &account)
    }

    /// Add to account balance.
    pub fn add_balance(&self, address: &Address, amount: u64) -> Result<()> {
        let mut account = self.get_account(address)?;
        account.balance = account.balance.saturating_add(amount);
        self.put_account(address, &account)
    }

    /// Subtract from account balance.
    /// Returns error if insufficient balance.
    pub fn sub_balance(&self, address: &Address, amount: u64) -> Result<()> {
        let mut account = self.get_account(address)?;
        if account.balance < amount {
            return Err(StorageError::InsufficientBalance {
                address: *address,
                required: amount,
                available: account.balance,
            });
        }
        account.balance -= amount;
        self.put_account(address, &account)
    }

    /// Transfer balance from one account to another.
    pub fn transfer(&self, from: &Address, to: &Address, amount: u64) -> Result<()> {
        self.sub_balance(from, amount)?;
        self.add_balance(to, amount)?;
        Ok(())
    }

    // =========================================================================
    // Nonce Operations
    // =========================================================================

    /// Get account nonce.
    pub fn get_nonce(&self, address: &Address) -> Result<u64> {
        Ok(self.get_account(address)?.nonce)
    }

    /// Increment nonce and return the old value.
    pub fn increment_nonce(&self, address: &Address) -> Result<u64> {
        let mut account = self.get_account(address)?;
        let old_nonce = account.nonce;
        account.nonce = account.nonce.saturating_add(1);
        self.put_account(address, &account)?;
        Ok(old_nonce)
    }

    // =========================================================================
    // Contract Code Storage
    // =========================================================================

    /// Store contract bytecode.
    /// Code is stored by its hash, enabling deduplication.
    pub fn put_code(&self, code_hash: &Hash, code: &[u8]) -> Result<()> {
        let key = Storage::code_key(code_hash);
        self.storage.put(key, &code.to_vec())
    }

    /// Retrieve contract bytecode by its hash.
    pub fn get_code(&self, code_hash: &Hash) -> Result<Option<Vec<u8>>> {
        let key = Storage::code_key(code_hash);
        self.storage.get(key)
    }

    /// Deploy a contract: store code and create contract account.
    /// Returns the code hash.
    pub fn deploy_contract(
        &self,
        address: &Address,
        code: &[u8],
        initial_balance: u64,
    ) -> Result<Hash> {
        // Hash the code
        let code_hash = hash(code);

        // Store the bytecode (by hash, enabling deduplication)
        self.put_code(&code_hash, code)?;

        // Create the contract account
        let mut account = Account::new_contract(code_hash);
        account.balance = initial_balance;
        self.put_account(address, &account)?;

        Ok(code_hash)
    }

    /// Get the code for a contract address.
    /// Returns None if the address is not a contract or has no code.
    pub fn get_code_for_address(&self, address: &Address) -> Result<Option<Vec<u8>>> {
        let account = self.get_account(address)?;
        match account.code_hash {
            Some(code_hash) => self.get_code(&code_hash),
            None => Ok(None),
        }
    }

    // =========================================================================
    // Contract Storage (SLOAD/SSTORE)
    // =========================================================================

    /// Read from contract storage (raw bytes).
    pub fn storage_get(&self, contract: &Address, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let db_key = Storage::contract_storage_key(contract, key);
        self.storage.get(db_key)
    }

    /// Write to contract storage (raw bytes).
    pub fn storage_put(&self, contract: &Address, key: &[u8], value: &[u8]) -> Result<()> {
        let db_key = Storage::contract_storage_key(contract, key);
        self.storage.put(db_key, &value.to_vec())
    }

    /// Delete from contract storage.
    pub fn storage_delete(&self, contract: &Address, key: &[u8]) -> Result<()> {
        let db_key = Storage::contract_storage_key(contract, key);
        self.storage.delete(db_key)
    }

    /// Read a 32-byte slot from contract storage (SLOAD).
    /// Returns zero bytes if the slot is uninitialized.
    pub fn sload(&self, contract: &Address, slot: &StorageSlot) -> Result<[u8; 32]> {
        match self.storage_get(contract, slot)? {
            Some(bytes) if bytes.len() == 32 => {
                let mut result = [0u8; 32];
                result.copy_from_slice(&bytes);
                Ok(result)
            }
            Some(_) => Err(StorageError::InvalidStorageValue),
            None => Ok([0u8; 32]), // Uninitialized slots are zero
        }
    }

    /// Write a 32-byte value to contract storage (SSTORE).
    pub fn sstore(&self, contract: &Address, slot: &StorageSlot, value: &[u8; 32]) -> Result<()> {
        self.storage_put(contract, slot, value)
    }

    // =========================================================================
    // State Root Computation
    // =========================================================================

    /// Compute the state root from all accounts.
    ///
    /// This iterates all accounts and computes a merkle root.
    /// Note: This is a naive O(n) implementation. Production systems use
    /// incremental structures like Merkle Patricia Tries.
    pub fn compute_state_root(&self) -> Result<Hash> {
        let mut account_hashes = Vec::new();

        // Iterate over all accounts (prefix scan)
        let prefix = b"account:";
        for result in self.storage.inner().scan_prefix(prefix) {
            let (key, value) = result.map_err(StorageError::Database)?;

            // Hash the key-value pair together
            let pair_hash = hash(&[key.as_ref(), value.as_ref()].concat());
            account_hashes.push(pair_hash);
        }

        // Sort for deterministic ordering
        // (sled iteration order may vary, so we sort by hash)
        account_hashes.sort_by(|a, b| a.0.cmp(&b.0));

        // Compute merkle root
        Ok(merkle_root(&account_hashes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Storage {
        Storage::open_temporary().unwrap()
    }

    #[test]
    fn test_account_crud() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let address = Address([0xAA; 20]);

        // Initially doesn't exist
        assert!(!state.account_exists(&address).unwrap());

        // Get returns default
        let account = state.get_account(&address).unwrap();
        assert_eq!(account.balance, 0);
        assert_eq!(account.nonce, 0);

        // Put an account
        let mut account = Account::new_user(1000);
        account.nonce = 5;
        state.put_account(&address, &account).unwrap();

        // Now exists
        assert!(state.account_exists(&address).unwrap());

        // Retrieve it
        let retrieved = state.get_account(&address).unwrap();
        assert_eq!(retrieved.balance, 1000);
        assert_eq!(retrieved.nonce, 5);
    }

    #[test]
    fn test_balance_operations() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let address = Address([0xBB; 20]);

        // Set balance
        state.set_balance(&address, 1000).unwrap();
        assert_eq!(state.get_balance(&address).unwrap(), 1000);

        // Add balance
        state.add_balance(&address, 500).unwrap();
        assert_eq!(state.get_balance(&address).unwrap(), 1500);

        // Subtract balance
        state.sub_balance(&address, 300).unwrap();
        assert_eq!(state.get_balance(&address).unwrap(), 1200);

        // Insufficient balance
        let result = state.sub_balance(&address, 2000);
        assert!(matches!(
            result,
            Err(StorageError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn test_transfer() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let alice = Address([0xAA; 20]);
        let bob = Address([0xBB; 20]);

        state.set_balance(&alice, 1000).unwrap();
        state.transfer(&alice, &bob, 300).unwrap();

        assert_eq!(state.get_balance(&alice).unwrap(), 700);
        assert_eq!(state.get_balance(&bob).unwrap(), 300);
    }

    #[test]
    fn test_nonce_operations() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let address = Address([0xCC; 20]);

        assert_eq!(state.get_nonce(&address).unwrap(), 0);

        let old = state.increment_nonce(&address).unwrap();
        assert_eq!(old, 0);
        assert_eq!(state.get_nonce(&address).unwrap(), 1);

        let old = state.increment_nonce(&address).unwrap();
        assert_eq!(old, 1);
        assert_eq!(state.get_nonce(&address).unwrap(), 2);
    }

    #[test]
    fn test_contract_deployment() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let address = Address([0xDD; 20]);
        let bytecode = b"contract bytecode here";

        let code_hash = state.deploy_contract(&address, bytecode, 100).unwrap();

        // Account exists and is a contract
        let account = state.get_account(&address).unwrap();
        assert!(account.is_contract());
        assert_eq!(account.balance, 100);
        assert_eq!(account.code_hash, Some(code_hash));

        // Code is retrievable
        let code = state.get_code(&code_hash).unwrap().unwrap();
        assert_eq!(code, bytecode);

        // Code for address works too
        let code = state.get_code_for_address(&address).unwrap().unwrap();
        assert_eq!(code, bytecode);
    }

    #[test]
    fn test_contract_storage() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let contract = Address([0xEE; 20]);

        // Raw storage operations
        state.storage_put(&contract, b"key1", b"value1").unwrap();
        let value = state.storage_get(&contract, b"key1").unwrap().unwrap();
        assert_eq!(value, b"value1");

        // SLOAD/SSTORE with 32-byte slots
        let slot: StorageSlot = [0u8; 32];
        let value: [u8; 32] = [0xAB; 32];

        state.sstore(&contract, &slot, &value).unwrap();
        let loaded = state.sload(&contract, &slot).unwrap();
        assert_eq!(loaded, value);

        // Uninitialized slot returns zeros
        let empty_slot: StorageSlot = [0xFF; 32];
        let loaded = state.sload(&contract, &empty_slot).unwrap();
        assert_eq!(loaded, [0u8; 32]);
    }

    #[test]
    fn test_storage_isolation() {
        let storage = setup();
        let state = StateManager::new(&storage);
        let contract_a = Address([0xAA; 20]);
        let contract_b = Address([0xBB; 20]);

        // Both contracts write to "slot 0"
        let slot: StorageSlot = [0u8; 32];
        let value_a: [u8; 32] = [0xAA; 32];
        let value_b: [u8; 32] = [0xBB; 32];

        state.sstore(&contract_a, &slot, &value_a).unwrap();
        state.sstore(&contract_b, &slot, &value_b).unwrap();

        // They're isolated
        assert_eq!(state.sload(&contract_a, &slot).unwrap(), value_a);
        assert_eq!(state.sload(&contract_b, &slot).unwrap(), value_b);
    }

    #[test]
    fn test_compute_state_root() {
        let storage = setup();
        let state = StateManager::new(&storage);

        // Empty state
        let root1 = state.compute_state_root().unwrap();

        // Add an account
        let alice = Address([0xAA; 20]);
        state.set_balance(&alice, 1000).unwrap();
        let root2 = state.compute_state_root().unwrap();

        // Roots should differ
        assert_ne!(root1, root2);

        // Same state = same root (deterministic)
        let root3 = state.compute_state_root().unwrap();
        assert_eq!(root2, root3);

        // Change state = different root
        state.set_balance(&alice, 2000).unwrap();
        let root4 = state.compute_state_root().unwrap();
        assert_ne!(root3, root4);
    }
}
