//! Integration tests for the English Auction smart contract.
//!
//! These tests assemble `contracts/auction.asm` and run it through the VM
//! with an in-memory storage backend to verify every code-path:
//!
//!   1.  Initialization (first call sets seller, reserve price, end time, etc.)
//!   2.  Valid first bid
//!   3.  Bid below reserve price → revert
//!   4.  Bid below minimum increment → revert
//!   5.  Outbid credits previous bidder's pending_returns
//!   6.  Outbid bidder withdraws
//!   7.  Withdraw with nothing → revert
//!   8.  Double withdraw → revert
//!   9.  Auto-finalization after time expiry
//!   10. Bid after auction ended → revert
//!   11. Seller withdrawal after auction ends
//!   12. Seller double-withdraw → revert
//!   13. Seller cannot withdraw before auction ends
//!   14. Multiple outbid refunds accumulate
//!   15. Full happy-path end-to-end
//!   16. Gas consumption is bounded

use minichain_assembler::assemble;
use minichain_core::Address;
use minichain_vm::{ExecutionResult, StorageBackend, Vm, VmError};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// In-memory storage backend
// ---------------------------------------------------------------------------

/// Simple in-memory storage backend for testing.
#[derive(Clone, Default, Debug)]
struct TestStorage {
    slots: HashMap<[u8; 32], [u8; 32]>,
}

impl StorageBackend for TestStorage {
    fn sload(&self, key: &[u8; 32]) -> [u8; 32] {
        self.slots.get(key).copied().unwrap_or([0u8; 32])
    }
    fn sstore(&mut self, key: &[u8; 32], value: &[u8; 32]) {
        self.slots.insert(*key, *value);
    }
}

impl TestStorage {
    /// Read a u64 value from the given storage slot number, matching the
    /// VM's convention (u64 lives in bytes 24..32 big-endian of the 32-byte
    /// slot).
    fn get_u64(&self, slot: u64) -> u64 {
        let mut key = [0u8; 32];
        key[24..32].copy_from_slice(&slot.to_be_bytes());
        let val = self.sload(&key);
        u64::from_be_bytes(val[24..32].try_into().unwrap())
    }
}

// ---------------------------------------------------------------------------
// RefCell wrapper so we can recover storage after VM execution
// ---------------------------------------------------------------------------

/// Thin wrapper that delegates `StorageBackend` calls through a raw pointer
/// to a `RefCell<TestStorage>` living on the caller's stack frame.
struct RefCellStorage(*const std::cell::RefCell<TestStorage>);

// SAFETY: tests are single-threaded; the pointer is valid for the duration of
// `run_auction`.
unsafe impl Send for RefCellStorage {}
unsafe impl Sync for RefCellStorage {}

impl StorageBackend for RefCellStorage {
    fn sload(&self, key: &[u8; 32]) -> [u8; 32] {
        unsafe { (*self.0).borrow().sload(key) }
    }
    fn sstore(&mut self, key: &[u8; 32], value: &[u8; 32]) {
        unsafe { (*self.0).borrow_mut().sstore(key, value) }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an [`Address`] whose first 8 bytes (little-endian) equal `id`.
/// The VM's `CALLER` opcode reads the first 8 bytes of the 20-byte address
/// via `u64::from_le_bytes`, so this lets us predict the value in R4.
fn make_address(id: u64) -> Address {
    let mut bytes = [0u8; 20];
    bytes[..8].copy_from_slice(&id.to_le_bytes());
    Address::from_bytes(bytes)
}

/// Compile the auction contract (source at workspace root).
fn compile_auction() -> Vec<u8> {
    // `cargo test` may run from the crate root (crates/vm) or workspace root.
    let paths = ["../../contracts/auction.asm", "contracts/auction.asm"];
    let source = paths
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
        .expect("could not find contracts/auction.asm");
    assemble(&source).expect("failed to assemble auction.asm")
}

/// Execute the auction bytecode in a fresh VM with the supplied context.
/// Returns `Ok((result, storage))` on a successful HALT, or
/// `Err((VmError, storage))` when the VM reverts / errors.
fn run_auction(
    bytecode: &[u8],
    storage: TestStorage,
    caller: Address,
    call_value: u64,
    timestamp: u64,
) -> Result<(ExecutionResult, TestStorage), (VmError, TestStorage)> {
    let contract_addr = make_address(0xC0C0);
    let mut vm = Vm::new_with_context(
        bytecode.to_vec(),
        5_000_000,
        caller,
        contract_addr,
        call_value,
        1,
        timestamp,
    );

    let storage_cell = std::cell::RefCell::new(storage);
    let wrapper = RefCellStorage(std::ptr::addr_of!(storage_cell));
    vm.set_storage(Box::new(wrapper));

    match vm.run() {
        Ok(result) => Ok((result, storage_cell.into_inner())),
        Err(e) => Err((e, storage_cell.into_inner())),
    }
}

/// Unwrap a successful run or panic with a descriptive message.
fn run_ok(
    bytecode: &[u8],
    storage: TestStorage,
    caller: Address,
    call_value: u64,
    timestamp: u64,
    msg: &str,
) -> (ExecutionResult, TestStorage) {
    run_auction(bytecode, storage, caller, call_value, timestamp)
        .unwrap_or_else(|(e, _)| panic!("{}: {:?}", msg, e))
}

/// Unwrap a failed run or panic with a descriptive message.
fn run_err(
    bytecode: &[u8],
    storage: TestStorage,
    caller: Address,
    call_value: u64,
    timestamp: u64,
    msg: &str,
) -> (VmError, TestStorage) {
    match run_auction(bytecode, storage, caller, call_value, timestamp) {
        Err(pair) => pair,
        Ok(_) => panic!("{}: expected error but VM succeeded", msg),
    }
}

// ---------------------------------------------------------------------------
// Constants shared across tests
// ---------------------------------------------------------------------------

const SELLER_ID: u64 = 0xAA;
const BIDDER1_ID: u64 = 0xBB;
const BIDDER2_ID: u64 = 0xCC;
const BIDDER3_ID: u64 = 0xDD;

const RESERVE_PRICE: u64 = 100;
const MIN_INCREMENT: u64 = 10;
const AUCTION_DURATION: u64 = 3600;
const INIT_TIMESTAMP: u64 = 1_000_000;

/// Compute the storage-slot key for a bidder's pending_returns entry.
fn pending_key(caller_id: u64) -> u64 {
    let mask: u64 = 0x00FFFFFFFFFFFFFF;
    10 + (caller_id & mask)
}

// ===========================================================================
// Tests
// ===========================================================================

// ---- 1. Initialization ----------------------------------------------------

#[test]
fn test_init_sets_storage() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);

    let (result, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );

    assert!(result.success, "init must HALT successfully");
    assert_eq!(st.get_u64(0), SELLER_ID, "slot 0 = seller");
    assert_eq!(st.get_u64(4), RESERVE_PRICE, "slot 4 = reserve_price");
    assert_eq!(
        st.get_u64(3),
        INIT_TIMESTAMP + AUCTION_DURATION,
        "slot 3 = end_time"
    );
    assert_eq!(st.get_u64(6), MIN_INCREMENT, "slot 6 = min_increment");
    assert_eq!(st.get_u64(5), 0, "slot 5 = ended (false)");
    assert_eq!(result.logs, vec![SELLER_ID]);
}

// ---- 2. Valid first bid ----------------------------------------------------

#[test]
fn test_first_bid_valid() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (result, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid");

    assert!(result.success);
    assert_eq!(st.get_u64(1), 150, "highest_bid = 150");
    assert_eq!(st.get_u64(2), BIDDER1_ID, "highest_bidder = bidder1");
    assert_eq!(result.logs, vec![150, BIDDER1_ID]);
}

// ---- 3. Bid below reserve price -> revert ----------------------------------

#[test]
fn test_bid_below_reserve_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );

    let (e, _) = run_err(
        &bc,
        st,
        bidder1,
        50,
        INIT_TIMESTAMP + 10,
        "bid below reserve",
    );
    assert_eq!(e, VmError::Reverted);
}

// ---- 4. Bid below minimum increment -> revert ------------------------------

#[test]
fn test_bid_below_min_increment_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid1");

    // +5 above 150, but min_increment=10 -> revert
    let (e, _) = run_err(
        &bc,
        st,
        bidder2,
        155,
        INIT_TIMESTAMP + 20,
        "bid below increment",
    );
    assert_eq!(e, VmError::Reverted);
}

// ---- 5. Outbid credits previous bidder's pending_returns --------------------

#[test]
fn test_outbid_credits_pending_returns() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid1");
    let (result, st) = run_ok(&bc, st, bidder2, 200, INIT_TIMESTAMP + 20, "bid2");

    assert!(result.success);
    assert_eq!(st.get_u64(1), 200, "highest_bid = 200");
    assert_eq!(st.get_u64(2), BIDDER2_ID, "highest_bidder = bidder2");
    assert_eq!(
        st.get_u64(pending_key(BIDDER1_ID)),
        150,
        "bidder1 pending = 150"
    );
    assert_eq!(result.logs, vec![BIDDER1_ID, 200, BIDDER2_ID]);
}

// ---- 6. Outbid bidder withdraws --------------------------------------------

#[test]
fn test_withdraw_by_outbid_bidder() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid1");
    let (_, st) = run_ok(&bc, st, bidder2, 200, INIT_TIMESTAMP + 20, "bid2");
    let (result, st) = run_ok(&bc, st, bidder1, 0, INIT_TIMESTAMP + 30, "withdraw");

    assert!(result.success);
    assert_eq!(st.get_u64(pending_key(BIDDER1_ID)), 0, "pending zeroed");
    assert_eq!(result.logs, vec![150, BIDDER1_ID]);
}

// ---- 7. Withdraw with nothing -> revert ------------------------------------

#[test]
fn test_withdraw_nothing_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );

    let (e, _) = run_err(&bc, st, bidder1, 0, INIT_TIMESTAMP + 10, "withdraw nothing");
    assert_eq!(e, VmError::Reverted);
}

// ---- 8. Double withdraw -> revert ------------------------------------------

#[test]
fn test_double_withdraw_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid1");
    let (_, st) = run_ok(&bc, st, bidder2, 200, INIT_TIMESTAMP + 20, "bid2");
    let (_, st) = run_ok(&bc, st, bidder1, 0, INIT_TIMESTAMP + 30, "withdraw1");

    let (e, _) = run_err(&bc, st, bidder1, 0, INIT_TIMESTAMP + 40, "double withdraw");
    assert_eq!(e, VmError::Reverted);
}

// ---- 9. Auto-finalization after time expiry --------------------------------

#[test]
fn test_auto_finalize_on_time_expiry() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let end_time = INIT_TIMESTAMP + AUCTION_DURATION;

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid");
    assert_eq!(st.get_u64(5), 0, "ended == 0 before expiry");

    // bidder1 with value=0 after end -> auto-finalize sets ended=1,
    // then withdraw path (no pending) -> revert. Storage still mutated.
    let (e, st) = run_err(&bc, st, bidder1, 0, end_time + 1, "auto-finalize");
    assert_eq!(e, VmError::Reverted);
    assert_eq!(st.get_u64(5), 1, "ended flag set after expiry");
}

// ---- 10. Bid after auction ended -> revert ---------------------------------

#[test]
fn test_bid_after_ended_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);
    let end_time = INIT_TIMESTAMP + AUCTION_DURATION;

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid");
    // Seller calls after end -> auto-finalize + seller_withdraw succeeds
    let (result, st) = run_ok(&bc, st, seller, 0, end_time + 1, "seller finalizes");
    assert!(result.success);
    assert_eq!(st.get_u64(5), 1);

    // Bidder2 tries to bid after ended -> revert
    let (e, _) = run_err(&bc, st, bidder2, 300, end_time + 100, "bid after ended");
    assert_eq!(e, VmError::Reverted);
}

// ---- 11. Seller withdrawal after auction ends ------------------------------

#[test]
fn test_seller_withdraw_after_end() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let end_time = INIT_TIMESTAMP + AUCTION_DURATION;

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 200, INIT_TIMESTAMP + 10, "bid");
    let (result, st) = run_ok(&bc, st, seller, 0, end_time + 10, "seller withdraw");

    assert!(result.success);
    assert!(result.logs.contains(&200), "winning bid in logs");
    assert!(result.logs.contains(&SELLER_ID), "seller addr in logs");
    assert_eq!(st.get_u64(1), 0, "highest_bid cleared");
}

// ---- 12. Seller double-withdraw -> revert ----------------------------------

#[test]
fn test_seller_double_withdraw_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let end_time = INIT_TIMESTAMP + AUCTION_DURATION;

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 200, INIT_TIMESTAMP + 10, "bid");
    let (_, st) = run_ok(&bc, st, seller, 0, end_time + 10, "seller withdraw 1");

    let (e, _) = run_err(&bc, st, seller, 0, end_time + 20, "seller double withdraw");
    assert_eq!(e, VmError::Reverted);
}

// ---- 13. Seller cannot withdraw before auction ends -------------------------

#[test]
fn test_seller_withdraw_before_end_reverts() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 200, INIT_TIMESTAMP + 10, "bid");

    let (e, _) = run_err(
        &bc,
        st,
        seller,
        0,
        INIT_TIMESTAMP + 100,
        "seller early withdraw",
    );
    assert_eq!(e, VmError::Reverted);
}

// ---- 14. Multiple outbid refunds accumulate --------------------------------

#[test]
fn test_multiple_bids_accumulate_pending_returns() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);
    let bidder3 = make_address(BIDDER3_ID);

    let (_, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    let (_, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 10, "bid1");
    let (_, st) = run_ok(&bc, st, bidder2, 200, INIT_TIMESTAMP + 20, "bid2");
    let (_, st) = run_ok(&bc, st, bidder3, 300, INIT_TIMESTAMP + 30, "bid3");

    assert_eq!(st.get_u64(pending_key(BIDDER1_ID)), 150);
    assert_eq!(st.get_u64(pending_key(BIDDER2_ID)), 200);
    assert_eq!(st.get_u64(1), 300, "highest_bid = 300");
    assert_eq!(st.get_u64(2), BIDDER3_ID, "highest_bidder = bidder3");
}

// ---- 15. Full happy-path E2E -----------------------------------------------

#[test]
fn test_full_happy_path() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);
    let bidder1 = make_address(BIDDER1_ID);
    let bidder2 = make_address(BIDDER2_ID);
    let end_time = INIT_TIMESTAMP + AUCTION_DURATION;

    // 1. Init
    let (r, st) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    assert!(r.success);

    // 2. Bidder1 bids 150
    let (r, st) = run_ok(&bc, st, bidder1, 150, INIT_TIMESTAMP + 100, "bid1");
    assert!(r.success);

    // 3. Bidder2 bids 250
    let (r, st) = run_ok(&bc, st, bidder2, 250, INIT_TIMESTAMP + 200, "bid2");
    assert!(r.success);

    // 4. Bidder1 withdraws refund (150)
    let (r, st) = run_ok(&bc, st, bidder1, 0, INIT_TIMESTAMP + 300, "b1 withdraw");
    assert!(r.success);
    assert!(r.logs.contains(&150), "withdrawn amount logged");

    // 5. Auction ends -> seller claims 250
    let (r, st) = run_ok(&bc, st, seller, 0, end_time + 1, "seller claim");
    assert!(r.success);
    assert!(r.logs.contains(&250), "winning bid logged");
    assert_eq!(st.get_u64(1), 0, "highest_bid cleared");
    assert_eq!(st.get_u64(5), 1, "ended = true");
}

// ---- 16. Gas usage is bounded ----------------------------------------------

#[test]
fn test_gas_usage_is_bounded() {
    let bc = compile_auction();
    let seller = make_address(SELLER_ID);

    let (result, _) = run_ok(
        &bc,
        TestStorage::default(),
        seller,
        0,
        INIT_TIMESTAMP,
        "init",
    );
    assert!(result.gas_used > 0, "gas must be consumed");
    assert!(result.gas_used < 500_000, "init should use < 500k gas");
}
