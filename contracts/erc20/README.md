# ERC20 Token Contract for Minichain

A working ERC20-style token contract for minichain assembly, including:

- real VM-backed execution during block production
- read-only queries via `minichain call --query`
- deploy-time initialization for owner, metadata, and initial supply
- Bun end-to-end tests under [`contracts/erc20/test`](/home/pavitra/Projects/minichain/contracts/erc20/test)

## What This Contract Supports

The contract exposes these selectors:

| Function | Selector |
| --- | --- |
| `totalSupply()` | `0x00` |
| `balanceOf(addressId)` | `0x01` |
| `transfer(to, amount)` | `0x02` |
| `approve(spender, amount)` | `0x03` |
| `transferFrom(from, to, amount)` | `0x04` |
| `allowance(owner, spender)` | `0x05` |
| `mint(to, amount)` | `0x06` |
| `burn(amount)` | `0x07` |
| `name()` | `0x08` |
| `symbol()` | `0x09` |
| `decimals()` | `0x0A` |
| `init(owner, name, symbol, decimals, initialTo, initialSupply)` | `0xFF` |

Notes:

- `mint` is owner-only.
- `burn` destroys the caller's tokens.
- `name` and `symbol` are stored as up to 8 ASCII characters packed into a `u64`.
- Account arguments are not full 20-byte addresses. The contract uses the same `u64` view the VM exposes from the first 8 bytes of the minichain address in little-endian form.

## Quick Start

### 1. Initialize a chain and create accounts

```bash
cargo run --release -- init --authorities 1

cargo run --release -- account new --name alice
cargo run --release -- account new --name bob
cargo run --release -- account new --name charlie
```

### 2. Fund the accounts with native MIC

The CLI now requires signer aliases with `@`.

```bash
cargo run --release -- account mint --from @authority_0 --to <ALICE_ADDR> --amount 1000000
cargo run --release -- account mint --from @authority_0 --to <BOB_ADDR> --amount 500000
cargo run --release -- account mint --from @authority_0 --to <CHARLIE_ADDR> --amount 500000
```

### 3. Convert addresses to contract `u64` ids

The ERC20 contract expects the first 8 address bytes interpreted as a little-endian `u64`.

Example in Bun:

```ts
function addressToId(address: string): bigint {
  const bytes = Buffer.from(address.replace(/^0x/i, ""), "hex");
  return bytes.readBigUInt64LE(0);
}
```

### 4. Deploy with metadata and owner setup

Deployment accepts init calldata through `--init-data`. The payload is regular call encoding for selector `0xFF`.

```bash
cargo run --release -- deploy \
  --from @alice \
  --source contracts/erc20/src/erc20.asm \
  --init-data <HEX_INIT_CALLDATA> \
  --gas-limit 400000
```

Then produce a block:

```bash
cargo run --release -- block produce --authority authority_0
```

The deploy command prints:

```text
Contract Address: 0x...
```

## CLI Usage

### Read-only calls

Use `--query` for getters. Query mode executes immediately and prints a stable result line:

```bash
cargo run --release -- call \
  --query \
  --from @alice \
  --to 0x... \
  --data <HEX_CALLDATA>
```

Output includes:

```text
Result: 0x...
```

### State-changing calls

State-changing calls are still transactions. Submit the call, then produce a block:

```bash
cargo run --release -- call \
  --from @alice \
  --to 0x... \
  --data <HEX_CALLDATA> \
  --gas-limit 250000

cargo run --release -- block produce --authority authority_0
```

## Calldata Encoding

Calldata is packed as:

```text
[selector:u64 little-endian][arg1:u64 little-endian][arg2:u64 little-endian]...
```

That means selector `0x08` must be passed as:

```text
0800000000000000
```

Amount `1000` must be passed as:

```text
e803000000000000
```

### Example helper

```ts
function encodeWord(value: number | bigint): string {
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setBigUint64(0, BigInt(value), true);
  return Buffer.from(buffer).toString("hex");
}

function encodeCall(selector: number, args: readonly (number | bigint)[]): string {
  return [encodeWord(selector), ...args.map(encodeWord)].join("");
}
```

## Example Calls

Assume:

- `TOKEN_ADDR=0x...`
- `ALICE_ID`, `BOB_ID`, and `CHARLIE_ID` are the `u64` address ids derived from their real addresses

### Query metadata

```bash
cargo run --release -- call --query --from @alice --to $TOKEN_ADDR --data 0800000000000000
cargo run --release -- call --query --from @alice --to $TOKEN_ADDR --data 0900000000000000
cargo run --release -- call --query --from @alice --to $TOKEN_ADDR --data 0a00000000000000
```

### Mint tokens to Alice

```bash
cargo run --release -- call \
  --from @alice \
  --to $TOKEN_ADDR \
  --data <selector 0x06 + ALICE_ID + 1000> \
  --gas-limit 250000

cargo run --release -- block produce --authority authority_0
```

### Check balances

```bash
cargo run --release -- call \
  --query \
  --from @alice \
  --to $TOKEN_ADDR \
  --data <selector 0x01 + ALICE_ID>
```

### Approve and transferFrom

```bash
# Alice approves Bob for 200
cargo run --release -- call --from @alice --to $TOKEN_ADDR --data <selector 0x03 + BOB_ID + 200> --gas-limit 250000
cargo run --release -- block produce --authority authority_0

# Bob transfers 150 from Alice to Charlie
cargo run --release -- call --from @bob --to $TOKEN_ADDR --data <selector 0x04 + ALICE_ID + CHARLIE_ID + 150> --gas-limit 250000
cargo run --release -- block produce --authority authority_0
```

### Burn tokens

```bash
cargo run --release -- call --from @alice --to $TOKEN_ADDR --data <selector 0x07 + 100> --gas-limit 250000
cargo run --release -- block produce --authority authority_0
```

## Testing

The contract is exercised by the Bun E2E suite:

```bash
cargo build --release -p minichain-cli
cd contracts/erc20
bun test
```

The test flow covers:

- deploy with metadata initialization
- owner mint success
- non-owner mint failure
- transfer
- self-transfer
- approval and allowance tracking
- `transferFrom`
- burn
- final supply and balance checks

## Files

- [`src/erc20.asm`](/home/pavitra/Projects/minichain/contracts/erc20/src/erc20.asm): contract implementation
- [`test/contract-client.ts`](/home/pavitra/Projects/minichain/contracts/erc20/test/contract-client.ts): Bun-side encoding and deployment helpers
- [`test/e2e.test.ts`](/home/pavitra/Projects/minichain/contracts/erc20/test/e2e.test.ts): end-to-end contract tests
- [`ERC20_DESIGN.md`](/home/pavitra/Projects/minichain/contracts/erc20/ERC20_DESIGN.md): storage layout and design notes
