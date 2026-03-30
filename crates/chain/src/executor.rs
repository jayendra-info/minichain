//! Block execution engine.
//!
//! Executes transactions in blocks and updates the world state.

use minichain_core::{Account, Address, Block, Hash, Transaction};
use minichain_storage::StateManager;
use minichain_vm::{StorageBackend, Vm, VmError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

/// Fixed-size deployment header that prefixes runtime bytecode length.
const DEPLOY_HEADER_BYTES: usize = 4;

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

    #[error("invalid deployment payload: {0}")]
    InvalidDeploymentPayload(String),

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

#[derive(Debug, Clone)]
pub struct ContractQueryResult {
    /// Whether execution succeeded.
    pub success: bool,
    /// Gas used by the query.
    pub gas_used: u64,
    /// Raw return bytes read back from VM memory.
    pub return_data: Vec<u8>,
    /// Error message if execution failed.
    pub error: Option<String>,
}

/// Inputs for executing a read-only contract query.
pub struct ContractQuery<'a> {
    /// Caller address exposed to the VM via `CALLER`.
    pub caller: Address,
    /// Calldata loaded into VM memory before execution.
    pub data: &'a [u8],
    /// Call value exposed via `CALLVALUE`.
    pub call_value: u64,
    /// Maximum gas available to the query execution.
    pub gas_limit: u64,
    /// Current block number for context opcodes.
    pub block_number: u64,
    /// Current block timestamp for context opcodes.
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
struct VmExecution {
    success: bool,
    gas_used: u64,
    return_data: Vec<u8>,
    error: Option<String>,
}

/// Encode deployment data as: `[runtime_len: u32][runtime_code][init_data]`.
pub fn encode_deployment_payload(runtime_code: &[u8], init_data: &[u8]) -> Vec<u8> {
    let mut payload =
        Vec::with_capacity(DEPLOY_HEADER_BYTES + runtime_code.len() + init_data.len());
    payload.extend_from_slice(&(runtime_code.len() as u32).to_le_bytes());
    payload.extend_from_slice(runtime_code);
    payload.extend_from_slice(init_data);
    payload
}

/// Decode deployment data into `(runtime_code, init_data)`.
pub fn decode_deployment_payload(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    if data.len() < DEPLOY_HEADER_BYTES {
        return Err(ExecutionError::InvalidDeploymentPayload(
            "missing runtime bytecode length header".to_string(),
        ));
    }

    let runtime_len = u32::from_le_bytes(
        data[..DEPLOY_HEADER_BYTES]
            .try_into()
            .expect("slice length checked"),
    ) as usize;
    let runtime_end = DEPLOY_HEADER_BYTES + runtime_len;
    if runtime_len == 0 || runtime_end > data.len() {
        return Err(ExecutionError::InvalidDeploymentPayload(
            "runtime bytecode length is invalid".to_string(),
        ));
    }

    Ok((
        data[DEPLOY_HEADER_BYTES..runtime_end].to_vec(),
        data[runtime_end..].to_vec(),
    ))
}

#[derive(Clone)]
struct OverlayStorage<'a> {
    state: &'a StateManager<'a>,
    contract: Address,
    writes: Rc<RefCell<HashMap<[u8; 32], [u8; 32]>>>,
}

impl<'a> OverlayStorage<'a> {
    fn new(state: &'a StateManager<'a>, contract: Address) -> Self {
        Self {
            state,
            contract,
            writes: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl StorageBackend for OverlayStorage<'_> {
    fn sload(&self, key: &[u8; 32]) -> [u8; 32] {
        if let Some(value) = self.writes.borrow().get(key) {
            return *value;
        }
        self.state
            .sload(&self.contract, key)
            .expect("contract storage read should not fail")
    }

    fn sstore(&mut self, key: &[u8; 32], value: &[u8; 32]) {
        self.writes.borrow_mut().insert(*key, *value);
    }
}

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
        self.execute_transaction_with_context(tx, 0, 0)
    }

    /// Execute a single transaction with explicit block context for VM opcodes.
    pub fn execute_transaction_with_context(
        &self,
        tx: &Transaction,
        block_number: u64,
        timestamp: u64,
    ) -> Result<TransactionReceipt> {
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

        // Deduct max cost (value + gas) upfront
        self.state.sub_balance(sender, max_cost)?;

        // Execute based on transaction type
        let (success, gas_used, contract_address, error) = if tx.is_deploy() {
            self.execute_deploy(tx, block_number, timestamp)?
        } else if tx.is_call() {
            self.execute_call(tx, block_number, timestamp)?
        } else {
            // For transfer: just add value to recipient (sender already deducted in max_cost)
            self.execute_transfer_without_sender_deduction(tx)?
        };

        // Refund unused gas. Failed calls do not transfer call value, so only actual
        // gas spent is charged in that case.
        let charged_value = if success { tx.value } else { 0 };
        let refund = max_cost
            .saturating_sub(charged_value)
            .saturating_sub(gas_used.saturating_mul(tx.gas_price));
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

    /// Execute a contract in read-only mode against the current state.
    pub fn query_contract(
        &self,
        contract: &Address,
        query: ContractQuery<'_>,
    ) -> Result<ContractQueryResult> {
        let account = self.state.get_account(contract)?;
        if !account.is_contract() {
            return Ok(ContractQueryResult {
                success: false,
                gas_used: 0,
                return_data: Vec::new(),
                error: Some("contract not found or no code".to_string()),
            });
        }

        let code = self
            .state
            .get_code_for_address(contract)?
            .ok_or_else(|| ExecutionError::VmError("missing contract bytecode".to_string()))?;

        let execution = self.execute_contract_code(
            &code,
            contract,
            query.caller,
            query.call_value,
            query.gas_limit,
            query.data,
            query.block_number,
            query.timestamp,
            true,
        )?;

        Ok(ContractQueryResult {
            success: execution.success,
            gas_used: execution.gas_used,
            return_data: execution.return_data,
            error: execution.error,
        })
    }

    /// Execute transfer where sender balance is already deducted (just add to recipient).
    /// This is used when executor deducts max_cost (value + gas) upfront.
    fn execute_transfer_without_sender_deduction(
        &self,
        tx: &Transaction,
    ) -> Result<(bool, u64, Option<Address>, Option<String>)> {
        let to = tx.to.expect("transfer must have recipient");

        // Just add value to recipient (sender already deducted in max_cost)
        if tx.value > 0 {
            self.state.add_balance(&to, tx.value)?;
        }

        // Transfer uses 21,000 gas
        Ok((true, 21_000, None, None))
    }

    /// Execute a contract deployment transaction.
    fn execute_deploy(
        &self,
        tx: &Transaction,
        block_number: u64,
        timestamp: u64,
    ) -> Result<(bool, u64, Option<Address>, Option<String>)> {
        // Calculate contract address and unpack runtime/init payload
        let contract_addr = tx
            .contract_address()
            .expect("deploy must calculate address");
        let (runtime_code, init_data) = decode_deployment_payload(&tx.data)?;

        // Deployment gas is the base create cost plus per-byte code cost.
        let base_gas = 32_000 + (runtime_code.len() as u64 * 200);
        if tx.gas_limit < base_gas {
            return Ok((
                false,
                tx.gas_limit,
                None,
                Some(
                    VmError::OutOfGas {
                        required: base_gas,
                        remaining: tx.gas_limit,
                    }
                    .to_string(),
                ),
            ));
        }

        // Store bytecode by hash so later calls can load it by account code_hash.
        let code_hash = minichain_core::hash(&runtime_code);
        self.state.put_code(&code_hash, &runtime_code)?;

        // Run optional init calldata against the freshly created contract storage.
        let mut gas_used = base_gas;
        if !init_data.is_empty() {
            let init_execution = self.execute_contract_code(
                &runtime_code,
                &contract_addr,
                tx.from,
                0,
                tx.gas_limit.saturating_sub(base_gas),
                &init_data,
                block_number,
                timestamp,
                false,
            )?;
            gas_used = gas_used.saturating_add(init_execution.gas_used);
            if !init_execution.success {
                return Ok((false, gas_used, None, init_execution.error));
            }
        }

        // Only persist the account if the init execution succeeded.
        let mut contract_account = Account::new_contract(code_hash);
        contract_account.balance = tx.value;
        self.state.put_account(&contract_addr, &contract_account)?;

        Ok((true, gas_used, Some(contract_addr), None))
    }

    /// Execute a contract call transaction.
    fn execute_call(
        &self,
        tx: &Transaction,
        block_number: u64,
        timestamp: u64,
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

        // Load runtime bytecode and execute against a transactional storage overlay.
        let code = self
            .state
            .get_code_for_address(&contract_addr)?
            .ok_or_else(|| ExecutionError::VmError("missing contract bytecode".to_string()))?;

        let execution = self.execute_contract_code(
            &code,
            &contract_addr,
            tx.from,
            tx.value,
            tx.gas_limit,
            &tx.data,
            block_number,
            timestamp,
            false,
        )?;

        // Only credit transferred value to the contract if execution did not revert.
        if execution.success && tx.value > 0 {
            self.state.add_balance(&contract_addr, tx.value)?;
        }

        Ok((execution.success, execution.gas_used, None, execution.error))
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_contract_code(
        &self,
        code: &[u8],
        contract_addr: &Address,
        caller: Address,
        call_value: u64,
        gas_limit: u64,
        calldata: &[u8],
        block_number: u64,
        timestamp: u64,
        read_only: bool,
    ) -> Result<VmExecution> {
        // Buffer SSTORE writes in memory first so reverts and queries do not mutate state.
        let overlay = OverlayStorage::new(self.state, *contract_addr);
        let writes = overlay.writes.clone();

        let mut vm = Vm::new_with_context(
            code.to_vec(),
            gas_limit,
            caller,
            *contract_addr,
            call_value,
            block_number,
            timestamp,
        );
        vm.load_memory(0, calldata)
            .map_err(|err| ExecutionError::VmError(err.to_string()))?;
        vm.set_storage(Box::new(overlay));

        match vm.run() {
            Ok(result) => {
                // Commit storage changes only for successful state-changing executions.
                if !read_only {
                    self.commit_storage_writes(contract_addr, &writes.borrow())?;
                }
                Ok(VmExecution {
                    success: result.success,
                    gas_used: result.gas_used,
                    return_data: result.return_data,
                    error: None,
                })
            }
            Err(err) => Ok(VmExecution {
                success: false,
                gas_used: gas_limit.saturating_sub(vm.gas_remaining()),
                return_data: Vec::new(),
                error: Some(err.to_string()),
            }),
        }
    }

    fn commit_storage_writes(
        &self,
        contract_addr: &Address,
        writes: &HashMap<[u8; 32], [u8; 32]>,
    ) -> Result<()> {
        // Apply each buffered slot update to persistent contract storage.
        for (slot, value) in writes {
            self.state.sstore(contract_addr, slot, value)?;
        }
        Ok(())
    }

    /// Execute a block of transactions.
    pub fn execute_block(&self, block: &Block) -> Result<BlockExecutionResult> {
        let block_hash = block.hash();
        let mut receipts = Vec::new();
        let mut total_gas_used = 0;

        for tx in &block.transactions {
            let receipt = self.execute_transaction_with_context(
                tx,
                block.header.height,
                block.header.timestamp,
            )?;
            total_gas_used += receipt.gas_used;
            receipts.push(receipt);
        }

        let state_root = self.state.compute_state_root()?;

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
    use minichain_assembler::assemble;
    use minichain_core::Keypair;
    use minichain_storage::Storage;

    fn setup() -> (Storage, Keypair) {
        (Storage::open_temporary().unwrap(), Keypair::generate())
    }

    fn sample_contract() -> Vec<u8> {
        assemble(
            r#"
            .entry main
            main:
                LOADI R0, 0
                LOAD64 R1, R0
                LOADI R2, 0
                EQ R3, R1, R2
                LOADI R4, get_value
                JUMPI R3, R4

                LOADI R2, 1
                EQ R3, R1, R2
                LOADI R4, set_value
                JUMPI R3, R4

                REVERT

            get_value:
                LOADI R5, 1
                SLOAD R6, R5
                LOADI R7, 0
                STORE64 R7, R6
                HALT

            set_value:
                LOADI R5, 8
                LOAD64 R6, R5
                LOADI R7, 1
                SSTORE R7, R6
                HALT
        "#,
        )
        .unwrap()
    }

    #[test]
    fn test_deployment_payload_roundtrip() {
        let runtime = vec![1, 2, 3];
        let init = vec![4, 5];
        let payload = encode_deployment_payload(&runtime, &init);
        let (decoded_runtime, decoded_init) = decode_deployment_payload(&payload).unwrap();
        assert_eq!(decoded_runtime, runtime);
        assert_eq!(decoded_init, init);
    }

    #[test]
    fn test_execute_transfer() {
        let (storage, keypair) = setup();
        let state = StateManager::new(&storage);

        let from = keypair.address();
        let to = Address::from_bytes([2u8; 20]);

        state.set_balance(&from, 100_000).unwrap();
        state
            .put_account(&from, &Account::new_user(100_000))
            .unwrap();

        let tx = Transaction::transfer(from, to, 1000, 0, 1).signed(&keypair);

        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&tx).unwrap();

        assert!(receipt.success);
        assert_eq!(receipt.gas_used, 21_000);
        assert_eq!(state.get_balance(&from).unwrap(), 78_000);
        assert_eq!(state.get_balance(&to).unwrap(), 1000);
        assert_eq!(state.get_nonce(&from).unwrap(), 1);
    }

    #[test]
    fn test_execute_deployment_stores_code() {
        let (storage, keypair) = setup();
        let state = StateManager::new(&storage);
        let from = keypair.address();
        let runtime = sample_contract();
        let payload = encode_deployment_payload(&runtime, &[]);

        state.set_balance(&from, 1_000_000).unwrap();
        state
            .put_account(&from, &Account::new_user(1_000_000))
            .unwrap();

        let tx = Transaction::deploy(from, payload, 0, 200_000, 1).signed(&keypair);

        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&tx).unwrap();

        assert!(receipt.success);
        let contract_addr = receipt.contract_address.unwrap();
        let contract = state.get_account(&contract_addr).unwrap();
        assert!(contract.is_contract());
        assert_eq!(
            state.get_code_for_address(&contract_addr).unwrap(),
            Some(runtime)
        );
    }

    #[test]
    fn test_execute_call_updates_storage() {
        let (storage, keypair) = setup();
        let state = StateManager::new(&storage);
        let from = keypair.address();
        let runtime = sample_contract();
        let payload = encode_deployment_payload(&runtime, &[]);

        state.set_balance(&from, 1_000_000).unwrap();
        state
            .put_account(&from, &Account::new_user(1_000_000))
            .unwrap();

        let deploy = Transaction::deploy(from, payload, 0, 200_000, 1).signed(&keypair);
        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&deploy).unwrap();
        let contract_addr = receipt.contract_address.unwrap();

        let mut calldata = 1u64.to_le_bytes().to_vec();
        calldata.extend_from_slice(&42u64.to_le_bytes());
        let call =
            Transaction::call(from, contract_addr, calldata, 0, 1, 100_000, 1).signed(&keypair);
        let call_receipt = executor.execute_transaction(&call).unwrap();

        assert!(call_receipt.success);

        let query = executor
            .query_contract(
                &contract_addr,
                ContractQuery {
                    caller: from,
                    data: &0u64.to_le_bytes(),
                    call_value: 0,
                    gas_limit: 100_000,
                    block_number: 1,
                    timestamp: 1,
                },
            )
            .unwrap();

        assert!(query.success);
        assert_eq!(
            u64::from_le_bytes(query.return_data[..8].try_into().unwrap()),
            42
        );
    }

    #[test]
    fn test_query_does_not_commit_storage_writes() {
        let (storage, keypair) = setup();
        let state = StateManager::new(&storage);
        let from = keypair.address();
        let runtime = sample_contract();
        let payload = encode_deployment_payload(&runtime, &[]);

        state.set_balance(&from, 1_000_000).unwrap();
        state
            .put_account(&from, &Account::new_user(1_000_000))
            .unwrap();

        let deploy = Transaction::deploy(from, payload, 0, 200_000, 1).signed(&keypair);
        let executor = Executor::new(&state);
        let receipt = executor.execute_transaction(&deploy).unwrap();
        let contract_addr = receipt.contract_address.unwrap();

        let mut calldata = [1u64.to_le_bytes().to_vec(), 99u64.to_le_bytes().to_vec()].concat();
        let query = executor
            .query_contract(
                &contract_addr,
                ContractQuery {
                    caller: from,
                    data: &calldata,
                    call_value: 0,
                    gas_limit: 100_000,
                    block_number: 1,
                    timestamp: 1,
                },
            )
            .unwrap();
        assert!(query.success);
        calldata = 0u64.to_le_bytes().to_vec();
        let value = executor
            .query_contract(
                &contract_addr,
                ContractQuery {
                    caller: from,
                    data: &calldata,
                    call_value: 0,
                    gas_limit: 100_000,
                    block_number: 1,
                    timestamp: 1,
                },
            )
            .unwrap();
        assert_eq!(
            u64::from_le_bytes(value.return_data[..8].try_into().unwrap()),
            0
        );
    }
}
