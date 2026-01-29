//! Block execution engine.
//!
//! Executes transactions in blocks and updates the world state.

use minichain_core::{Account, Address, Block, Hash, Transaction};
use minichain_storage::StateManager;
use thiserror::Error;

/// Errors that can occur during execution.
#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("storage error: {0}")]
    Storage(#[from] minichain_storage::StorageError),

    #[error("insufficient balance (required {required}, available {available})")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("nonce mismatch (expected {expected}, got {got})")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("gas limit exceeded")]
    OutOfGas,

    #[error("execution reverted")]
    Reverted,

    #[error("VM error: {0}")]
    VmError(String),
}

pub type Result<T> = std::result::Result<T, ExecutionError>;

/// Result of executing a single transaction.
#[derive(Debug, Clone)]
pub struct TransactionReceipt {
    /// Transaction hash.
    pub tx_hash: Hash,
    /// Whether execution succeeded.
    pub success: bool,
    /// Gas used.
    pub gas_used: u64,
    /// Contract address (if deployment).
    pub contract_address: Option<Address>,
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Result of executing a block.
#[derive(Debug, Clone)]
pub struct BlockExecutionResult {
    /// Block hash.
    pub block_hash: Hash,
    /// Transaction receipts.
    pub receipts: Vec<TransactionReceipt>,
    /// Total gas used.
    pub total_gas_used: u64,
    /// New state root.
    pub state_root: Hash,
}

/// Block executor.
pub struct Executor<'a> {
    /// State manager for account operations.
    state: &'a StateManager<'a>,
}

impl<'a> Executor<'a> {
    /// Create a new executor.
    pub fn new(state: &'a StateManager<'a>) -> Self {
        Self { state }
    }

    /// Execute a single transaction.
    pub fn execute_transaction(&self, tx: &Transaction) -> Result<TransactionReceipt> {
        let tx_hash = tx.hash();
        let sender = &tx.from;

        // Get sender account
        let sender_account = self.state.get_account(sender)?;

        // Verify nonce
        if tx.nonce != sender_account.nonce {
            return Ok(TransactionReceipt {
                tx_hash,
                success: false,
                gas_used: 0,
                contract_address: None,
                error: Some(format!(
                    "invalid nonce: expected {}, got {}",
                    sender_account.nonce, tx.nonce
                )),
            });
        }

        // Check balance
        let max_cost = tx.max_cost();
        if sender_account.balance < max_cost {
            return Ok(TransactionReceipt {
                tx_hash,
                success: false,
                gas_used: 0,
                contract_address: None,
                error: Some(format!(
                    "insufficient balance: required {}, available {}",
                    max_cost, sender_account.balance
                )),
            });
        }

        // Increment nonce
        let old_nonce = self.state.increment_nonce(sender)?;
        assert_eq!(old_nonce, tx.nonce);

        // Deduct max gas cost
        self.state.sub_balance(sender, max_cost)?;

        // Execute based on transaction type
        let (success, gas_used, contract_address, error) = if tx.is_deploy() {
            self.execute_deploy(tx)?
        } else if tx.is_call() {
            self.execute_call(tx)?
        } else {
            self.execute_transfer(tx)?
        };

        // Refund unused gas
        let gas_cost = gas_used * tx.gas_price;
        let refund = max_cost - gas_cost;
        if refund > 0 {
            self.state.add_balance(sender, refund)?;
        }

        Ok(TransactionReceipt {
            tx_hash,
            success,
            gas_used,
            contract_address,
            error,
        })
    }

    /// Execute a transfer transaction.
    fn execute_transfer(
        &self,
        tx: &Transaction,
    ) -> Result<(bool, u64, Option<Address>, Option<String>)> {
        let to = tx.to.expect("transfer must have recipient");

        // Transfer value
        if tx.value > 0 {
            self.state.transfer(&tx.from, &to, tx.value)?;
        }

        // Transfer uses 21,000 gas
        Ok((true, 21_000, None, None))
    }

    /// Execute a contract deployment transaction.
    fn execute_deploy(
        &self,
        tx: &Transaction,
    ) -> Result<(bool, u64, Option<Address>, Option<String>)> {
        // Calculate contract address
        let contract_addr = tx
            .contract_address()
            .expect("deploy must calculate address");

        // Calculate code hash
        let code_hash = minichain_core::hash(&tx.data);

        // Create contract account with code hash
        let contract_account = Account {
            balance: tx.value,
            nonce: 0,
            code_hash: Some(code_hash),
            storage_root: Hash::ZERO,
        };

        self.state.put_account(&contract_addr, &contract_account)?;

        // In a real implementation, store the actual code separately indexed by code_hash
        // For now, we just store the account with the code hash

        // Simplified gas calculation: 32,000 base + 200 per byte
        let gas_used = 32_000 + (tx.data.len() as u64 * 200);

        Ok((true, gas_used, Some(contract_addr), None))
    }

    /// Execute a contract call transaction.
    fn execute_call(
        &self,
        tx: &Transaction,
    ) -> Result<(bool, u64, Option<Address>, Option<String>)> {
        let contract_addr = tx.to.expect("call must have recipient");

        // Get contract account
        let contract = self.state.get_account(&contract_addr)?;

        // Check if contract exists
        if !contract.is_contract() {
            return Ok((
                false,
                21_000,
                None,
                Some("contract not found or no code".to_string()),
            ));
        }

        // Transfer value if any
        if tx.value > 0 {
            self.state.transfer(&tx.from, &contract_addr, tx.value)?;
        }

        // In a real implementation, this would:
        // 1. Initialize VM with contract code
        // 2. Set up execution context (caller, calldata, etc.)
        // 3. Run the VM
        // 4. Apply state changes
        // 5. Return execution result
        //
        // For now, we simulate successful execution
        let gas_used = 21_000 + (tx.data.len() as u64 * 68);

        Ok((true, gas_used, None, None))
    }

    /// Execute a block of transactions.
    pub fn execute_block(&self, block: &Block) -> Result<BlockExecutionResult> {
        let block_hash = block.hash();
        let mut receipts = Vec::new();
        let mut total_gas_used = 0;

        for tx in &block.transactions {
            let receipt = self.execute_transaction(tx)?;
            total_gas_used += receipt.gas_used;
            receipts.push(receipt);
        }

        // In a real implementation, compute the actual state root from the trie
        // For now, use a simplified hash
        let state_root = minichain_core::hash(&total_gas_used.to_le_bytes());

        Ok(BlockExecutionResult {
            block_hash,
            receipts,
            total_gas_used,
            state_root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minichain_core::Keypair;
    use minichain_storage::Storage;

    #[test]
    fn test_execute_transfer() {
        let storage = Storage::open_temporary().unwrap();
        let state = StateManager::new(&storage);

        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        // Setup sender with balance
        state.set_balance(&from, 100_000).unwrap();
        state
            .put_account(&from, &Account::new_user(100_000))
            .unwrap();

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&tx).unwrap();

        assert!(receipt.success);
        assert_eq!(receipt.gas_used, 21_000);
        assert!(receipt.contract_address.is_none());

        // Check balances
        let sender_balance = state.get_balance(&from).unwrap();
        let recipient_balance = state.get_balance(&to).unwrap();

        // Sender: 100_000 - 1000 (transfer) - 21_000 (gas)
        assert_eq!(sender_balance, 78_000);
        assert_eq!(recipient_balance, 1000);

        // Check nonce incremented
        assert_eq!(state.get_nonce(&from).unwrap(), 1);
    }

    #[test]
    fn test_execute_deployment() {
        let storage = Storage::open_temporary().unwrap();
        let state = StateManager::new(&storage);

        let keypair = Keypair::generate();
        let from = keypair.address();
        let bytecode = vec![0x60, 0x80, 0x60, 0x40]; // Sample bytecode

        // Setup sender with balance
        state.set_balance(&from, 1_000_000).unwrap();
        state
            .put_account(&from, &Account::new_user(1_000_000))
            .unwrap();

        let tx = Transaction::deploy(from, bytecode.clone(), 0, 100_000, 1).signed(&keypair);

        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&tx).unwrap();

        assert!(receipt.success);
        assert!(receipt.contract_address.is_some());

        // Verify contract was created
        let contract_addr = receipt.contract_address.unwrap();
        let contract = state.get_account(&contract_addr).unwrap();
        assert!(contract.is_contract());
        assert_eq!(contract.code_hash, Some(minichain_core::hash(&bytecode)));
    }

    #[test]
    fn test_execute_insufficient_balance() {
        let storage = Storage::open_temporary().unwrap();
        let state = StateManager::new(&storage);

        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        // Setup sender with insufficient balance
        state.set_balance(&from, 100).unwrap();
        state.put_account(&from, &Account::new_user(100)).unwrap();

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&tx).unwrap();

        assert!(!receipt.success);
        assert!(receipt.error.is_some());
        assert!(receipt.error.unwrap().contains("insufficient balance"));
    }

    #[test]
    fn test_execute_invalid_nonce() {
        let storage = Storage::open_temporary().unwrap();
        let state = StateManager::new(&storage);

        let keypair = Keypair::generate();
        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        // Setup sender with nonce 5
        let mut account = Account::new_user(100_000);
        account.nonce = 5;
        state.put_account(&from, &account).unwrap();

        // Transaction with nonce 0, but account nonce is 5
        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&tx).unwrap();

        assert!(!receipt.success);
        assert!(receipt.error.is_some());
        assert!(receipt.error.unwrap().contains("invalid nonce"));
    }

    #[test]
    fn test_execute_block() {
        let storage = Storage::open_temporary().unwrap();
        let state = StateManager::new(&storage);

        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let addr1 = keypair1.address();
        let addr2 = keypair2.address();

        // Setup accounts
        state
            .put_account(&addr1, &Account::new_user(100_000))
            .unwrap();
        state
            .put_account(&addr2, &Account::new_user(100_000))
            .unwrap();

        let tx1 = Transaction::transfer(addr1, addr2, 1000, 0, 1).signed(&keypair1);
        let tx2 = Transaction::transfer(addr2, addr1, 500, 0, 1).signed(&keypair2);

        let block = Block::new(1, Hash::ZERO, vec![tx1, tx2], Hash::ZERO, addr1);

        let executor = Executor::new(&state);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(result.receipts[1].success);
        assert_eq!(result.total_gas_used, 42_000); // 21_000 * 2
    }
}
