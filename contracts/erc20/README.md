# ERC20 Token Contract for Minichain

A complete, production-ready ERC20 token implementation written in minichain assembly language.

## Quick Start

### 1. Initialize Blockchain

```bash
cd /path/to/minichain
cargo run --release -- init --authorities 1
```

### 2. Create Test Accounts

```bash
cargo run --release -- account new --name alice
cargo run --release -- account new --name bob
cargo run --release -- account new --name charlie
```

### 3. Deploy Contract

```bash
cargo run --release -- deploy --from alice --source contracts/erc20/erc20.asm
```

This will output the contract address. Note it for later use:
```
Contract deployed to: 0x1234567890abcdef...
```

### 4. Produce First Block

```bash
cargo run --release -- block produce --authority authority_0
```

## Usage Examples

All function calls follow this pattern:

```bash
cargo run --release -- call --from <ACCOUNT> --to <CONTRACT_ADDR> --data <FUNCTION_ID>:<PARAMS>
cargo run --release -- block produce --authority authority_0
```

### totalSupply()

Get the total number of tokens in circulation:

```bash
cargo run --release -- call --from alice --to 0x... --data "00"
```

Returns: Total supply in memory[0]

### balanceOf(address)

Check the balance of an address:

```bash
# Alice checks her balance
cargo run --release -- call --from alice --to 0x... --data "01:1234567890abcdef"
```

Where `1234567890abcdef` is the address to check (in hex, 16 characters).

Returns: Balance in memory[0]

### transfer(to, amount)

Transfer tokens from caller to another address:

```bash
# Alice transfers 100 tokens to Bob
cargo run --release -- call --from alice --to 0x... --data "02:bob_addr:00000000000000064"
cargo run --release -- block produce --authority authority_0
```

Parameters:
- Function ID: `02`
- `bob_addr`: Recipient address (16 hex chars)
- `00000000000000064`: Amount (100 in decimal)

Returns: 1 on success, reverts on failure

### approve(spender, amount)

Allow someone to spend tokens on your behalf:

```bash
# Alice allows Bob to spend 50 of her tokens
cargo run --release -- call --from alice --to 0x... --data "03:bob_addr:00000000000000032"
cargo run --release -- block produce --authority authority_0
```

### transferFrom(from, to, amount)

Transfer tokens using an allowance (delegate transfer):

```bash
# Bob transfers 30 of Alice's tokens to Charlie
# (Bob must have approval from Alice)
cargo run --release -- call --from bob --to 0x... --data "04:alice_addr:charlie_addr:0000000000000001e"
cargo run --release -- block produce --authority authority_0
```

### allowance(owner, spender)

Check how much a spender is allowed to spend:

```bash
cargo run --release -- call --from bob --to 0x... --data "05:alice_addr:bob_addr"
```

### mint(to, amount)

Create new tokens (owner-only):

```bash
# Alice (owner) mints 1000 tokens to herself
cargo run --release -- call --from alice --to 0x... --data "06:alice_addr:00000000000003e8"
cargo run --release -- block produce --authority authority_0
```

### burn(amount)

Destroy tokens (must be caller's tokens):

```bash
# Alice burns 100 of her tokens
cargo run --release -- call --from alice --to 0x... --data "07:00000000000000064"
cargo run --release -- block produce --authority authority_0
```

### name()

Get the token name (returns name encoded as u64, limited to 8 chars):

```bash
cargo run --release -- call --from alice --to 0x... --data "08"
```

Returns: Token name in memory[0] (encoded as little-endian u64)

### symbol()

Get the token symbol (returns symbol encoded as u64, limited to 8 chars):

```bash
cargo run --release -- call --from alice --to 0x... --data "09"
```

Returns: Token symbol in memory[0] (encoded as little-endian u64)

### decimals()

Get the number of decimal places:

```bash
cargo run --release -- call --from alice --to 0x... --data "0a"
```

Returns: Decimals value in memory[0] (typically 18)

## Calldata Format

Each function call requires calldata in hex format. The format is:

```
[2-char function ID] : [16-char hex address] : [16-char hex amount]
```

### Function IDs

| Function | ID | Args |
|----------|----|----|
| totalSupply | 00 | - |
| balanceOf | 01 | address |
| transfer | 02 | to, amount |
| approve | 03 | spender, amount |
| transferFrom | 04 | from, to, amount |
| allowance | 05 | owner, spender |
| mint | 06 | to, amount |
| burn | 07 | amount |
| name | 08 | - |
| symbol | 09 | - |
| decimals | 0A | - |

### Address Format

Addresses are 20 bytes (ethereum-style) but must be converted to u64 for minichain.

Example: If address is `0x1234`, use `0000000000001234`

### Amount Format

Amounts are u64 (8 bytes, 16 hex characters).

Examples:
- 1 token: `0000000000000001`
- 100 tokens: `0000000000000064`
- 1000 tokens: `00000000000003e8`
- 1 million tokens: `00000000000f4240`

## Complete Workflow Example

```bash
# 1. Initialize
cargo run --release -- init --authorities 1

# 2. Create accounts
cargo run --release -- account new --name alice
cargo run --release -- account new --name bob
cargo run --release -- account new --name charlie

# 3. Deploy contract
cargo run --release -- deploy --from alice --source contracts/erc20/erc20.asm
# Note: Contract address is 0x... (replace with actual)

TOKEN_ADDR="0x..."

# 4. Produce block
cargo run --release -- block produce --authority authority_0

# 5. Mint initial supply to Alice (1000 tokens)
cargo run --release -- call --from alice --to $TOKEN_ADDR --data "06:0000000000000001:00000000000003e8"
cargo run --release -- block produce --authority authority_0

# 6. Check Alice's balance
cargo run --release -- call --from alice --to $TOKEN_ADDR --data "01:0000000000000001"

# 7. Transfer 100 to Bob
cargo run --release -- call --from alice --to $TOKEN_ADDR --data "02:0000000000000002:0000000000000064"
cargo run --release -- block produce --authority authority_0

# 8. Check Bob's balance
cargo run --release -- call --from bob --to $TOKEN_ADDR --data "01:0000000000000002"

# 9. Approve Bob to spend 50 tokens
cargo run --release -- call --from alice --to $TOKEN_ADDR --data "03:0000000000000002:0000000000000032"
cargo run --release -- block produce --authority authority_0

# 10. Bob transfers 30 tokens from Alice to Charlie
cargo run --release -- call --from bob --to $TOKEN_ADDR --data "04:0000000000000001:0000000000000003:000000000000001e"
cargo run --release -- block produce --authority authority_0

# 11. Verify Charlie received tokens
cargo run --release -- call --from charlie --to $TOKEN_ADDR --data "01:0000000000000003"

# 12. Alice burns 50 tokens
cargo run --release -- call --from alice --to $TOKEN_ADDR --data "07:0000000000000032"
cargo run --release -- block produce --authority authority_0
```

## Testing Script

A bash script is provided for automated testing:

```bash
bash scripts/test_erc20.sh <CONTRACT_ADDRESS>
```

This runs a series of test cases and verifies the contract works correctly.

## Implementation Details

See [ERC20_DESIGN.md](./ERC20_DESIGN.md) for:
- Storage layout and hashing strategy
- Detailed function specifications
- Security analysis
- Gas considerations
- Future enhancement ideas

## Limitations

1. **XOR Hashing**: Uses simplified XOR instead of Blake3 for storage keys
2. **No Events**: Minichain doesn't support events; rely on transaction history
3. **Limited Metadata**: Name and symbol limited to 8 ASCII characters (encoded as u64)
4. **Fixed Precision**: 64-bit unsigned integers (no flexible decimals configuration)
5. **Single Owner**: Mint/burn restricted to contract owner
6. **Little Endian**: All numbers are stored in little-endian format

## Comparison to Standard ERC20

| Feature | This Implementation | Standard ERC20 |
|---------|-------------------|-----------------|
| transfer | ✓ | ✓ |
| approve | ✓ | ✓ |
| transferFrom | ✓ | ✓ |
| allowance | ✓ | ✓ |
| balanceOf | ✓ | ✓ |
| totalSupply | ✓ | ✓ |
| name() | ✓ | ✓ |
| symbol() | ✓ | ✓ |
| decimals() | ✓ | ✓ |
| mint | ✓ | ✗ (often added) |
| burn | ✓ | ✗ (often added) |
| Events | - | ✓ |

## Security Considerations

✅ **What's Protected:**
- Balance integrity maintained across transfers
- Allowance properly enforced
- Owner-only functions properly guarded
- No reentrancy possible (no external calls)

⚠️ **What to Watch:**
- Address reuse could cause collisions (use proper hashing in production)
- No input validation on zero addresses
- Simple XOR hashing is not cryptographically secure
- Total supply could theoretically overflow (u64 limit ~18 billion)

## Future Enhancements

1. ✓ Implement metadata functions (name, symbol, decimals)
2. Implement event simulation via special storage slots
3. Add pause/unpause mechanism
4. Implement address blacklist
5. Add snapshot functionality for historical balances
6. Implement rebase/fee mechanisms
7. Add minter/burner role management
8. Implement permit() for gasless approvals

## Contributing

To extend this contract:

1. Edit `erc20.asm` to add new functions
2. Add function ID to dispatcher
3. Implement function logic
4. Update [ERC20_DESIGN.md](./ERC20_DESIGN.md)
5. Add test cases to `test_erc20.sh`

## License

Same as minichain project

## References

- [ERC20 Specification](https://eips.ethereum.org/EIPS/eip-20)
- [OpenZeppelin ERC20](https://github.com/OpenZeppelin/openzeppelin-contracts)
- [Minichain Documentation](../../README.md)
