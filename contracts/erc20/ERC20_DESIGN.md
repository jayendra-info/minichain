# ERC20 Token Contract - Design & Implementation Guide

## Overview

This contract is a VM-executed ERC20-style token for minichain assembly. It now runs against the real contract execution path in the chain runtime:

- deployment stores runtime bytecode by `code_hash`
- optional init calldata runs during deployment
- state-changing calls execute during block production
- read-only calls execute synchronously through `minichain call --query`

The implementation keeps the ERC20 surface small and educational while matching the actual minichain runtime behavior.

## Runtime Model

### Deployment path

Deployment transactions no longer contain only raw bytecode. The chain executor expects deployment data encoded as:

```text
[runtime_len: u32 little-endian][runtime_code][init_data]
```

At execution time:

1. the runtime bytecode is stored by `code_hash`
2. optional `init` calldata is executed against contract storage
3. the contract account is persisted only if init succeeds

This logic lives in [`crates/chain/src/executor.rs`](/home/pavitra/Projects/minichain/crates/chain/src/executor.rs).

### Call path

State-changing calls:

1. are submitted as transactions
2. enter the mempool
3. execute during block production
4. commit `SSTORE` writes only on success

Read-only calls:

1. use `minichain call --query`
2. run immediately against current state
3. do not change nonce or storage
4. return `Result: 0x...`

### Address model

The VM exposes `CALLER` and `ADDRESS` as `u64` values derived from the first 8 bytes of the 20-byte minichain address in little-endian order.

The ERC20 contract therefore uses `u64` account ids, not full 20-byte addresses, for:

- balances
- allowances
- owner identity
- transfer targets

## Contract Interface

### Public selectors

| Function | Selector | Args |
| --- | --- | --- |
| `totalSupply()` | `0x00` | none |
| `balanceOf(addressId)` | `0x01` | `addressId` |
| `transfer(to, amount)` | `0x02` | `to`, `amount` |
| `approve(spender, amount)` | `0x03` | `spender`, `amount` |
| `transferFrom(from, to, amount)` | `0x04` | `from`, `to`, `amount` |
| `allowance(owner, spender)` | `0x05` | `owner`, `spender` |
| `mint(to, amount)` | `0x06` | `to`, `amount` |
| `burn(amount)` | `0x07` | `amount` |
| `name()` | `0x08` | none |
| `symbol()` | `0x09` | none |
| `decimals()` | `0x0A` | none |
| `init(owner, name, symbol, decimals, initialTo, initialSupply)` | `0xFF` | 6 args |

### Behavioral notes

- `mint` is owner-only.
- `burn` burns the caller's own balance.
- `transferFrom` consumes allowance.
- `name` and `symbol` return packed ASCII `u64` values, up to 8 characters.
- Reverts are surfaced by the VM and storage writes are discarded.

## Storage Architecture

### Fixed slots

```text
Slot 0: total supply
Slot 1: owner id
Slot 2: name (packed ASCII u64)
Slot 3: symbol (packed ASCII u64)
Slot 4: decimals
```

### Dynamic slots

The current contract uses XOR-derived keys, not cryptographic hashing.

```text
BALANCE_MASK   = 0x1000000000000000
ALLOWANCE_MASK = 0x2000000000000000

balance_key(address)        = address XOR BALANCE_MASK
allowance_key(owner, spender) = owner XOR spender XOR ALLOWANCE_MASK
```

This matches the actual assembly in [`src/erc20.asm`](/home/pavitra/Projects/minichain/contracts/erc20/src/erc20.asm).

### Tradeoff

This is intentionally lightweight for minichain’s educational VM:

- simple and cheap to compute
- easy to inspect in assembly
- not collision-resistant like real map hashing

For production-grade behavior, the key-derivation strategy would need stronger hashing support in the VM or precompiles.

## Calldata Format

Every call uses packed little-endian `u64` words:

```text
[selector:u64 LE][arg1:u64 LE][arg2:u64 LE]...
```

Examples:

```text
name()               => 0800000000000000
decimals()           => 0a00000000000000
mint(to, 1000)       => 0600000000000000[to][e803000000000000]
burn(100)            => 07000000000000006400000000000000
```

The `init` selector uses the same format:

```text
ff00000000000000
[owner]
[name]
[symbol]
[decimals]
[initialTo]
[initialSupply]
```

## Function Logic

### totalSupply

- reads slot `0`
- writes the result to memory offset `0`
- halts

### balanceOf

- loads `addressId`
- computes `addressId XOR BALANCE_MASK`
- reads the slot with `SLOAD`
- returns the stored balance

### transfer

- uses `CALLER` as the sender id
- requires `amount > 0`
- requires sender balance `>= amount`
- debits sender and credits recipient

### approve

- uses `CALLER` as the owner id
- stores allowance at `owner XOR spender XOR ALLOWANCE_MASK`

### transferFrom

- uses `CALLER` as the spender id
- checks allowance and source balance
- decreases allowance
- moves tokens from `from` to `to`

### mint

- checks `CALLER == owner`
- requires `amount > 0`
- increases total supply
- credits recipient balance

### burn

- requires `amount > 0`
- requires caller balance `>= amount`
- debits caller balance
- reduces total supply

### init

The contract is initialized through deployment-time calldata instead of hard-coded metadata.

The init routine:

1. rejects repeated initialization if owner slot is already set
2. stores owner, name, symbol, and decimals
3. optionally sets initial supply and initial recipient balance if `initialSupply > 0`

## Query and Return Data

The current VM returns up to the first 8 bytes from memory starting at offset `0`.

This contract is designed around that constraint:

- getters write a single `u64` to memory offset `0`
- callers decode that as either:
  - integer (`totalSupply`, `balanceOf`, `allowance`, `decimals`)
  - ASCII-packed `u64` (`name`, `symbol`)

This is why all public getters currently return one word.

## Error Handling

The contract uses `REVERT` labels for invalid state transitions:

- zero-value transfers, mints, or burns
- insufficient balances
- insufficient allowance
- non-owner mint attempts
- repeated initialization
- unknown selectors

At the runtime level:

- storage writes are buffered in an overlay
- reverts discard the buffered writes
- query mode never commits writes

## Gas and Execution Notes

Important current runtime details:

- deploy base gas is `32_000 + 200 * bytecode_len`
- call transactions charge native MIC for max gas before execution
- unused gas is refunded after execution
- failed calls do not transfer call value

The contract itself keeps execution simple:

- no loops
- no external calls
- no logs/events
- one-word return values

## Testing Strategy

The Bun E2E suite under [`test/e2e.test.ts`](/home/pavitra/Projects/minichain/contracts/erc20/test/e2e.test.ts) validates:

- deploy with metadata initialization
- metadata getters
- zero initial balances
- owner mint success
- non-owner mint failure
- transfer
- self-transfer
- approve and allowance tracking
- `transferFrom`
- burn and supply reduction
- final balances after chained operations

Rust-side tests in the chain executor cover the runtime support this contract depends on:

- deployment payload encode/decode
- bytecode persistence
- storage-changing contract execution
- query mode not committing storage

## Limitations

1. Address ids are only 64 bits, derived from the first 8 bytes of the real address.
2. `name` and `symbol` are capped at 8 ASCII characters.
3. Mapping keys use XOR masks, not collision-resistant hashing.
4. There are no ERC20 events.
5. Allowance updates follow the simple overwrite pattern; there is no permit/increase/decrease allowance variant.

## Future Improvements

- support full 20-byte address handling in the VM
- return larger buffers from contracts for richer ABI behavior
- add event/log support
- use stronger storage key derivation
- add richer token extensions like pause, snapshot, or vote delegation
