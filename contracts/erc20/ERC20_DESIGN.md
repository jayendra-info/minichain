# ERC20 Token Contract - Design & Implementation Guide

## Overview

This is a complete ERC20 token implementation for minichain written in assembly. It supports the full standard ERC20 interface plus mint/burn functionality and metadata (name, symbol, decimals).

## Storage Architecture

### Layout

```
Slot 0 (TOTAL_SUPPLY_SLOT):
  64-bit unsigned integer representing total tokens in circulation

Slot 1 (OWNER_SLOT):
  64-bit address of the contract owner (can mint/burn)

Slot 2 (NAME_SLOT):
  64-bit value encoding token name (max 8 ASCII characters, little-endian)
  Example: "MyToken\0" encoded as u64

Slot 3 (SYMBOL_SLOT):
  64-bit value encoding token symbol (max 8 ASCII characters, little-endian)
  Example: "TKN\0\0\0\0\0" = 0x004E4B54 (little-endian)

Slot 4 (DECIMALS_SLOT):
  64-bit unsigned integer representing decimal places (e.g., 18 for Ethereum compatibility)

Dynamic Slots (Balances):
  Key = hash(address XOR BALANCE_SLOT_ID)
  Value = 64-bit balance for that address
  
Dynamic Slots (Allowances):
  Key = hash(owner_address XOR (spender_address XOR ALLOWANCE_SLOT_ID))
  Value = 64-bit allowance amount
```

### Hashing Strategy

For mapping-like storage, we use XOR-based hashing (simplified):

```
balance_key = address XOR 0  (BALANCE_SLOT_ID)
allowance_key = owner_address XOR (spender_address XOR 1)
```

This is a **simplified approach** for educational purposes. A real implementation would use cryptographic hashing with Blake3.

## Function Specifications

### 1. totalSupply()
**Function ID:** 0x00  
**Arguments:** None  
**Returns:** u64 (total supply)  
**Constraints:** None  
**Logic:**
- Load value from TOTAL_SUPPLY_SLOT
- Return to caller

### 2. balanceOf(address)
**Function ID:** 0x01  
**Arguments:** address (u64 at calldata offset 8)  
**Returns:** u64 (balance of address)  
**Constraints:** None  
**Logic:**
- Compute storage key = hash(address, BALANCE_SLOT_ID)
- Load from storage
- Return value (0 if not found)

### 3. transfer(to, amount)
**Function ID:** 0x02  
**Arguments:** to (8B@offset 8), amount (8B@offset 16)  
**Returns:** bool (success/failure)  
**Constraints:**
- amount > 0
- msg.sender balance >= amount

**Logic:**
- Verify conditions
- Deduct from sender balance
- Add to recipient balance
- Return true

### 4. approve(spender, amount)
**Function ID:** 0x03  
**Arguments:** spender (8B@8), amount (8B@16)  
**Returns:** bool (success)  
**Constraints:**
- amount >= 0

**Logic:**
- Set allowance[msg.sender][spender] = amount
- Return true

### 5. transferFrom(from, to, amount)
**Function ID:** 0x04  
**Arguments:** from (8B@8), to (8B@16), amount (8B@24)  
**Returns:** bool (success)  
**Constraints:**
- amount > 0
- allowance[from][msg.sender] >= amount
- from balance >= amount

**Logic:**
- Verify conditions
- Decrease allowance[from][msg.sender]
- Deduct from balance
- Add to recipient balance
- Return true

### 6. allowance(owner, spender)
**Function ID:** 0x05  
**Arguments:** owner (8B@8), spender (8B@16)  
**Returns:** u64 (allowed amount)  
**Logic:**
- Load allowance[owner][spender]
- Return value (0 if not set)

### 7. mint(to, amount)
**Function ID:** 0x06  
**Arguments:** to (8B@8), amount (8B@16)  
**Returns:** bool (success)  
**Constraints:**
- msg.sender == owner
- amount > 0

**Logic:**
- Verify caller is owner
- Increase total_supply
- Add to recipient balance
- Return true

### 8. burn(amount)
**Function ID:** 0x07  
**Arguments:** amount (8B@8)  
**Returns:** bool (success)  
**Constraints:**
- amount > 0
- msg.sender balance >= amount

**Logic:**
- Verify conditions
- Decrease balance
- Decrease total_supply
- Return true

### 9. name()
**Function ID:** 0x08  
**Arguments:** None  
**Returns:** u64 (token name encoded as little-endian ASCII)  
**Constraints:** None  
**Logic:**
- Load name from NAME_SLOT (slot 2)
- Return to caller

### 10. symbol()
**Function ID:** 0x09  
**Arguments:** None  
**Returns:** u64 (token symbol encoded as little-endian ASCII)  
**Constraints:** None  
**Logic:**
- Load symbol from SYMBOL_SLOT (slot 3)
- Return to caller (max 8 ASCII characters)

### 11. decimals()
**Function ID:** 0x0A  
**Arguments:** None  
**Returns:** u64 (number of decimal places)  
**Constraints:** None  
**Logic:**
- Load decimals from DECIMALS_SLOT (slot 4)
- Return to caller (typically 18 for ERC20 compatibility)

## Calldata Format

Functions accept calldata in the following format:

```
Byte 0-7:    Function ID (0-7)
Byte 8-15:   First parameter (address/amount)
Byte 16-23:  Second parameter (if applicable)
Byte 24-31:  Third parameter (if applicable)
```

### Example: transfer(0x123, 1000)

```
Calldata (hex): 02 00 00 00 00 00 00 00  | 23 01 00 00 00 00 00 00  | e8 03 00 00 00 00 00 00
                Function ID: 2           | to = 0x123             | amount = 1000
```

## Memory Layout

The contract uses memory for temporary storage:
- Offset 0-7: Return value (output)
- Offset 8+: Scratch space for computations

## Gas Considerations

- SLOAD: 100 gas (expensive, use sparingly)
- SSTORE: 5000 gas (very expensive)
- Arithmetic: 2-5 gas
- Comparisons: 2 gas

**Optimization Tips:**
- Cache frequently accessed values in registers
- Batch storage operations when possible
- Use XOR for key derivation (faster than hashing)

## Security Analysis

### Known Limitations

1. **Simplified Hashing:** Uses XOR instead of Blake3 for key derivation
   - Acceptable for educational purposes
   - In production, implement proper cryptographic hashing

2. **No Event Emission:** Minichain doesn't have native event logging
   - Could simulate via storage (inefficient)
   - Better: rely on transaction history for audit

3. **No SafeERC20:** Implementation doesn't include return value checks
   - Minichain REVERT on errors is sufficient

4. **Fixed 64-bit Precision:** Uses u64 integers directly
   - Decimals metadata defines UI representation (not actual precision)
   - Practical limit: ~18 quintillion tokens max

### Security Patterns Applied

✅ Check-Effects-Interactions (CEI) ordering  
✅ Revert on invalid state transitions  
✅ No external calls (single-contract)  
✅ Proper access control (owner-only operations)

## Implementation Statistics

- **Total Lines:** ~417 assembly instructions
- **Functions:** 11 core (6 standard ERC20 + 2 extended + 3 metadata) + dispatcher
- **Storage Slots:** 5 fixed + N dynamic
- **Bytecode Size:** ~897+ bytes (compiled)
- **Time to Deploy:** <1 second
- **Gas per Transfer:** ~5100 gas (estimate)
- **Compiler:** minichain assembler (2-pass label resolution)

## Testing Strategy

### Unit Tests (Rust)

1. Initialization & state
2. Balance operations
3. Allowance management
4. Mint/burn authorization
5. Edge cases (overflow, underflow)

### Integration Tests (CLI)

1. Happy path workflows
2. Access control violations
3. Insufficient funds
4. Multi-party scenarios

### Scenarios to Test

- Single transfer
- Multiple transfers preserving total supply
- Approve and transferFrom workflow
- Mint only by owner
- Burn reduces supply
- Allowance operations

## Code Organization

The assembly code is organized into logical sections:

```
├── Constants & Slots Definition
├── Main Dispatcher (routes to functions)
├── totalSupply() - Simple storage read
├── balanceOf() - Balance lookup
├── transfer() - Core transfer logic
├── approve() - Allowance setting
├── transferFrom() - Transfer with delegation
├── allowance() - Allowance lookup
├── mint() - Owner-only token creation
├── burn() - Owner-only token destruction
├── name() - Return token name metadata
├── symbol() - Return token symbol metadata
└── decimals() - Return decimal places metadata
```

Each function includes:
- Clear parameter documentation
- Input validation
- Storage key computation
- State updates with error handling

## Register Allocation Strategy

The implementation uses registers strategically:

- **R0-R5:** Parameter loading and temporary computation
- **R6-R12:** Key computation and balance tracking
- **R13-R15:** Storage operations and return values

This minimizes register pressure and keeps the code readable.

## Future Enhancements

1. **Event Simulation** - Store transfer logs in special storage slots
2. **SafeERC20** - Add return value validation
3. **Pause Mechanism** - Owner-controlled pause/unpause
4. **Blacklist** - Owner-controlled address blocking
5. **Rebase/Deflation** - Fee-on-transfer mechanism
6. **Snapshot** - Historical balance tracking
7. **Delegate/Voting** - Token holder voting power delegation

## Reference Implementation

This is based on OpenZeppelin's ERC20 implementation with simplifications for minichain's constraints:
- No events (minichain limitation)
- No external calls (single-contract focus)
- Fixed 64-bit precision (no decimals configuration)
- XOR-based key derivation (simplified hashing)
