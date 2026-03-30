# English Auction Contract - Design & Implementation Guide

## Overview

This contract is a VM-executed English auction for minichain assembly. The canonical source lives at `contracts/auction/src/auction.asm`, and the package includes both low-level VM tests and chain-level Bun tests.

The contract is intentionally simple:

- first execution initializes the seller and auction parameters
- bids are identified by non-zero `CALLVALUE`
- zero-value calls route to bidder refunds or seller withdrawal
- refund accounting is stored on-chain in contract storage

## Runtime Model

### Deployment path

Deployment uses the regular minichain contract deployment flow:

1. the assembly source is compiled to bytecode
2. a deployment transaction is submitted
3. block production executes the deployment and stores the runtime bytecode
4. the first call to the contract performs initialization

This auction contract does not use deploy-time init calldata, so a seller-side zero-value call is required after deployment to initialize the storage slots before bidders start sending value.

### Call path

State-changing calls are submitted with `minichain call` and only take effect after block production.

Because the contract has no getter selectors, chain-level tests validate behavior through observable balances and transaction effects rather than read-only queries.

## Contract Interface

The auction contract has an implicit interface based on call value and caller role:

- `CALLVALUE > 0` -> `bid()`
- `CALLVALUE == 0` and caller is seller -> `seller_withdraw()`
- `CALLVALUE == 0` and caller is not seller -> `withdraw()`

## Storage Architecture

### Fixed slots

```text
Slot 0: seller
Slot 1: highest_bid
Slot 2: highest_bidder
Slot 3: end_time
Slot 4: reserve_price
Slot 5: ended
Slot 6: min_bid_increment
```

### Dynamic slots

Pending refunds are stored from slot `10` upward:

```text
pending_returns_key(caller_id) = 10 + (caller_id & 0x00FFFFFFFFFFFFFF)
```

The contract uses the VM's `u64` caller view derived from the first 8 bytes of the address in little-endian order.

## Behavioral Notes

### Initialization

The first execution stores:

- seller = `CALLER`
- reserve price = `100`
- end time = `TIMESTAMP + 3600`
- minimum increment = `10`

### Bid flow

A valid bid must:

- happen before the auction is ended
- be at least `100`
- be at least `highest_bid + 10`

When a new highest bid replaces an existing one, the previous highest bid is credited into that bidder's `pending_returns` slot.

### Withdrawal flow

The contract's `withdraw()` and `seller_withdraw()` logic update internal storage and emit logs intended for the chain layer.

Current runtime limitation:

- the chain executor does not yet interpret those logs as native MIC transfers
- contract balance therefore increases with bids but does not decrease again on refund/proceeds calls

The package docs and E2E tests treat this as a known limitation, not as completed payout functionality.

## Testing Strategy

### Rust VM tests

`crates/vm/tests/auction_test.rs` remains the low-level logic suite. It covers:

- initialization
- valid and invalid bids
- pending refund accounting
- withdraw logic
- seller withdrawal logic
- auto-finalization logic
- gas bounds

### Bun E2E tests

`contracts/auction/test/e2e.test.ts` covers the real chain integration that is practical today:

- deploy from `contracts/auction/src/auction.asm`
- first funded call after deployment
- a later higher bid
- outbid accumulation into contract balance
- current no-payout behavior on withdrawal attempts

End-of-auction payout behavior is intentionally documented instead of exercised in Bun because the contract hardcodes a one-hour duration and the runtime payout bridge is not implemented yet.
