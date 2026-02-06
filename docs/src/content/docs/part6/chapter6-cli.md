---
title: "Chapter 6: Command Line Interface"
description: Building a full-featured CLI for minichain
---

# Chapter 6: Command Line Interface

In the previous chapters, we built all the core components of our blockchain: storage ([Chapter 2](/minichain/part2/chapter2-storage)), virtual machine ([Chapter 3](/minichain/part3/chapter3-vm)), assembler ([Chapter 4](/minichain/part4/chapter4-assembler)), and blockchain layer ([Chapter 5](/minichain/part5/chapter5-chain)). Now we need a way for users to interact with all this functionality.

This chapter builds a command-line interface (CLI) that ties everything together into a usable system. Think of the CLI as the "control panel" for your blockchain—a way to initialize chains, manage accounts, send transactions, deploy contracts, and produce blocks.

## What We're Building

| Command | Purpose | Example |
|---------|---------|---------|
| `init` | Initialize a new blockchain with genesis block | `minichain init --authorities 2` |
| `account` | Manage keypairs and query balances | `minichain account new --name alice` |
| `tx send` | Send value transfer transactions | `minichain tx send --from alice --to bob --amount 100` |
| `block` | Query and produce blocks | `minichain block produce --authority authority_0` |
| `deploy` | Compile and deploy contracts | `minichain deploy --from alice --source counter.asm` |
| `call` | Invoke deployed contracts | `minichain call --from alice --to 0xABC... --data 00` |

## Why a CLI?

Without a CLI, interacting with our blockchain would require:
- Writing Rust code to call library functions
- Manually managing keypairs and addresses
- Computing transaction hashes by hand
- Building and signing transactions programmatically

A CLI provides:
- **Accessibility**: Anyone can use the blockchain without writing code
- **Rapid prototyping**: Test contracts and transactions quickly
- **Developer experience**: Clear, documented commands with helpful error messages
- **Integration**: Easy to script and automate workflows

## CLI Architecture

Our CLI is built with the `clap` crate, which provides:
- Automatic help messages and argument parsing
- Subcommand structure (e.g., `minichain account new`)
- Type-safe command definitions
- Default values and validation

### Project Structure

```
crates/cli/
├── src/
│   ├── main.rs              # Entry point, argument parsing
│   └── commands/
│       ├── mod.rs            # Command dispatcher
│       ├── init.rs           # Chain initialization
│       ├── account.rs        # Account management
│       ├── tx.rs             # Transaction operations
│       ├── block.rs          # Block operations
│       ├── deploy.rs         # Contract deployment
│       └── call.rs           # Contract calls
└── Cargo.toml
```

Each command module follows the same pattern:
1. Define an `Args` struct with clap attributes
2. Implement a `run(args)` function
3. Parse inputs, load data from storage
4. Execute the operation
5. Display results with colored output

## Command Reference

### `minichain init`

Initialize a new blockchain with a genesis block.

**Usage:**
```bash
minichain init [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Directory to store blockchain data (default: `./data`)
- `-a, --authorities <N>`: Number of authorities to generate (default: `1`)
- `-b, --block-time <SECONDS>`: Target block time (default: `5`)

**What it does:**
1. Creates the data directory structure
2. Generates Ed25519 keypairs for authorities
3. Creates and signs the genesis block
4. Saves authority keypairs to `data/keys/authority_*.json`
5. Saves blockchain config to `data/config.json`

**Example:**
```bash
$ minichain init --authorities 2 --block-time 10

Initializing minichain...

Generating authorities...
  Authority 1: 0xf4a5e8c2b9d7f3a1e6c4b8d2f5a9e7c3b6d4f8a2
  Authority 2: 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7

✓  Created genesis block
    Hash: 0x8f2e5a9c7b3d1f6e4a2c8b5d9f7a3e1c6b4d8f2a
    Height: 0
✓  Saved authority 1 keypair to: data/keys/authority_0.json
✓  Saved authority 2 keypair to: data/keys/authority_1.json
✓  Saved config to: data/config.json

Chain initialized successfully!

Next steps:
  • Use minichain account new to create accounts
  • Use minichain tx send to send transactions
  • Use minichain block list to explore blocks
```

**Files created:**
```
data/
├── keys/
│   ├── authority_0.json
│   └── authority_1.json
├── config.json
└── sled/ (storage database)
```

### `minichain account`

Manage accounts and query balances.

#### `minichain account new`

Generate a new Ed25519 keypair and save it to disk.

**Usage:**
```bash
minichain account new [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Blockchain data directory (default: `./data`)
- `-n, --name <NAME>`: Name for the keypair file (default: auto-generated)

**Example:**
```bash
$ minichain account new --name alice

Generated new keypair:

  Address:     0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Public Key:  a3f5c9e7b2d8f4a6c1e3b9d7f5a2c8e4b6d9f3a7
  Private Key: 1f7a4c8e2b6d9f3a5c1e7b4d8f2a6c9e3b5d7f4a

✓  Saved to: data/keys/alice.json

Keep your private key safe!
```

The keypair JSON format:
```json
{
  "address": "0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a",
  "public_key": "a3f5c9e7b2d8f4a6c1e3b9d7f5a2c8e4b6d9f3a7",
  "private_key": "1f7a4c8e2b6d9f3a5c1e7b4d8f2a6c9e3b5d7f4a"
}
```

#### `minichain account balance`

Query the balance of an address.

**Usage:**
```bash
minichain account balance <ADDRESS>
```

**Example:**
```bash
$ minichain account balance 0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a

  Address: 0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Balance: 10000
```

#### `minichain account info`

Show detailed account information including nonce and contract status.

**Usage:**
```bash
minichain account info <ADDRESS>
```

**Example:**
```bash
$ minichain account info 0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a

Account Information:

  Address:      0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Balance:      10000
  Nonce:        5
  Is Contract:  No
```

For contracts:
```bash
$ minichain account info 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7

Account Information:

  Address:      0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
  Balance:      0
  Nonce:        0
  Is Contract:  Yes
  Code Hash:    0x8f2e5a9c7b3d1f6e4a2c8b5d9f7a3e1c6b4d8f2a
```

#### `minichain account list`

List all saved keypairs.

**Usage:**
```bash
minichain account list
```

**Example:**
```bash
$ minichain account list

Saved Keypairs:

  alice.json: 0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  bob.json: 0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e
  authority_0.json: 0xf4a5e8c2b9d7f3a1e6c4b8d2f5a9e7c3b6d4f8a2
  authority_1.json: 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
```

### `minichain tx send`

Send a value transfer transaction.

**Usage:**
```bash
minichain tx send [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Blockchain data directory (default: `./data`)
- `-f, --from <NAME>`: Sender keypair name (without `.json`)
- `-t, --to <ADDRESS>`: Recipient address (hex format)
- `-a, --amount <AMOUNT>`: Amount to send
- `--gas-price <PRICE>`: Gas price (default: `1`)

**What it does:**
1. Loads the sender's keypair from `data/keys/{from}.json`
2. Checks sender's balance against `amount + (21000 * gas_price)`
3. Creates and signs a transfer transaction
4. Submits it to the mempool
5. Returns the transaction hash

**Example:**
```bash
$ minichain tx send --from alice --to 0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e --amount 100

Sending transfer transaction...

  From:     0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  To:       0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e
  Amount:   100
  Nonce:    0
  Balance:  10000

✓  Transaction created
    Hash: 0x2c8f5a9e7b4d1f3a6c2e8b9d5f7a4c1e3b6d9f8a

✓  Transaction submitted to mempool

Transaction will be included in the next block.
Use minichain block produce to produce a block.
```

**Gas costs:**
- Base transaction: 21,000 gas
- Total cost: `amount + (21,000 * gas_price)`

### `minichain block`

Query and produce blocks.

#### `minichain block list`

List recent blocks.

**Usage:**
```bash
minichain block list [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Blockchain data directory (default: `./data`)
- `-c, --count <N>`: Number of blocks to show (default: `10`)

**Example:**
```bash
$ minichain block list --count 5

Recent Blocks:

  #5 8f2e5a9c7b3d1f6e (3 txs)
  #4 a7b3c9e5d1f4a8c2 (1 txs)
  #3 5c9e2b7d4f1a8c3e (2 txs)
  #2 f4a5e8c2b9d7f3a1 (0 txs)
  #1 3f8c2a6e9b5d1f4a (0 txs)
```

#### `minichain block info`

Show detailed information about a specific block.

**Usage:**
```bash
minichain block info <BLOCK_ID>
```

where `<BLOCK_ID>` can be either:
- Block height (e.g., `5`)
- Block hash (hex format, e.g., `0x8f2e5a9c7b3d1f6e...`)

**Example:**
```bash
$ minichain block info 5

Block Information:

  Height:       5
  Hash:         0x8f2e5a9c7b3d1f6e4a2c8b5d9f7a3e1c6b4d8f2a
  Parent Hash:  0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
  State Root:   0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e
  Timestamp:    1704067200
  Transactions: 3

Transactions:

  1. 2c8f5a9e7b4d1f3a
  2. f4a5e8c2b9d7f3a1
  3. 3f8c2a6e9b5d1f4a
```

#### `minichain block produce`

Produce a new block as an authority.

**Usage:**
```bash
minichain block produce [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Blockchain data directory (default: `./data`)
- `-a, --authority <NAME>`: Authority keypair name (without `.json`)

**What it does:**
1. Loads the authority's keypair
2. Verifies it's an authorized block producer
3. Collects pending transactions from mempool
4. Executes transactions and updates state
5. Creates and signs a new block
6. Appends it to the chain
7. Clears included transactions from mempool

**Example:**
```bash
$ minichain block produce --authority authority_0

Producing new block...

  Authority: 0xf4a5e8c2b9d7f3a1e6c4b8d2f5a9e7c3b6d4f8a2
  Current Height: 5

✓  Block produced
    Hash:   0x7d9f2a5c8e4b1f3a6c9d5f2a8c1e4b7d3f6a9c2e
    Height: 6
    Txs:    2
```

**Round-robin scheduling:**
Only the designated authority for the current height can produce a block:
```
Height 0 % 2 = 0  →  Authority 0
Height 1 % 2 = 1  →  Authority 1
Height 2 % 2 = 0  →  Authority 0
Height 3 % 2 = 1  →  Authority 1
...
```

### `minichain deploy`

Compile an assembly contract and deploy it to the blockchain.

**Usage:**
```bash
minichain deploy [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Blockchain data directory (default: `./data`)
- `-f, --from <NAME>`: Deployer keypair name (without `.json`)
- `-s, --source <PATH>`: Path to assembly source file
- `--gas-price <PRICE>`: Gas price (default: `1`)

**What it does:**
1. Reads and compiles the assembly source file
2. Loads the deployer's keypair
3. Checks balance against deployment cost
4. Creates and signs a deploy transaction
5. Submits it to the mempool
6. Calculates the contract address (deterministic from sender + nonce)

**Example:**
Create a simple counter contract (`counter.asm`):
```asm
.entry main

main:
    LOADI R0, 0          ; storage slot 0 = counter
    SLOAD R1, R0         ; load current value
    LOADI R2, 1
    ADD R1, R1, R2       ; increment
    SSTORE R0, R1        ; save back
    HALT
```

Deploy it:
```bash
$ minichain deploy --from alice --source counter.asm

Deploying contract...

  Compiling: counter.asm
✓  Compiled to 28 bytes

  Deployer:  0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Nonce:     1
  Balance:   9879

✓  Transaction created
    Hash: 0xd1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7b3d5f8a1

✓  Contract deployment submitted

  Contract Address: 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7

Transaction will be included in the next block.
Use minichain block produce to produce a block.
```

**Gas estimation:**
- Base: 21,000 gas
- Per byte of bytecode: 200 gas
- Example: 28-byte contract costs ~26,600 gas

**Contract address calculation:**
```
contract_address = first_20_bytes(hash(deployer_address || nonce))
```

This is deterministic—you know the contract address before the block is produced!

### `minichain call`

Invoke a deployed contract.

**Usage:**
```bash
minichain call [OPTIONS]
```

**Options:**
- `-d, --data-dir <PATH>`: Blockchain data directory (default: `./data`)
- `-f, --from <NAME>`: Caller keypair name (without `.json`)
- `-t, --to <ADDRESS>`: Contract address (hex format)
- `--data <HEX>`: Calldata (hex format, optional)
- `-a, --amount <AMOUNT>`: Amount to send (optional, default: `0`)
- `--gas-price <PRICE>`: Gas price (default: `1`)

**What it does:**
1. Loads the caller's keypair
2. Verifies the target address is a contract
3. Parses calldata from hex
4. Checks balance against `amount + gas_cost`
5. Creates and signs a call transaction
6. Submits it to the mempool

**Example:**
Call the counter contract deployed above:
```bash
$ minichain call --from alice --to 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7

Calling contract...

  Caller:    0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Contract:  0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
  Amount:    0
  Data:      0 bytes
  Nonce:     2
  Balance:   9853

✓  Transaction created
    Hash: 0xe2a5c9b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2

✓  Contract call submitted

Transaction will be included in the next block.
Use minichain block produce to produce a block.
```

With calldata:
```bash
$ minichain call --from alice --to 0xa7b3... --data 00000001
```

**Gas estimation:**
- Base: 21,000 gas
- Per byte of calldata: 68 gas (16 for zero byte, 68 for non-zero)
- Contract execution: depends on opcodes used

## Complete End-to-End Example

Let's walk through a complete workflow: initialize a chain, create accounts, fund them, deploy a contract, and interact with it.

### Step 1: Initialize the blockchain

```bash
$ minichain init --authorities 1

Initializing minichain...

Generating authorities...
  Authority 1: 0xf4a5e8c2b9d7f3a1e6c4b8d2f5a9e7c3b6d4f8a2

✓  Created genesis block
✓  Saved authority 1 keypair to: data/keys/authority_0.json
✓  Saved config to: data/config.json

Chain initialized successfully!
```

### Step 2: Create user accounts

```bash
$ minichain account new --name alice
Generated new keypair:
  Address: 0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
✓  Saved to: data/keys/alice.json

$ minichain account new --name bob
Generated new keypair:
  Address: 0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e
✓  Saved to: data/keys/bob.json
```

### Step 3: Fund accounts (manual state modification)

For testing, you can manually set balances by modifying state (in production, you'd have a faucet or initial allocation):

```rust
// In init.rs, after genesis block:
state.set_balance(&alice_addr, 1000000)?;
state.set_balance(&bob_addr, 1000000)?;
```

Or use a custom genesis block with pre-funded accounts.

### Step 4: Send a transfer

```bash
$ minichain tx send --from alice --to 0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e --amount 500

Sending transfer transaction...
  From:   0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  To:     0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e
  Amount: 500
✓  Transaction submitted to mempool
```

### Step 5: Produce a block

```bash
$ minichain block produce --authority authority_0

Producing new block...
  Authority: 0xf4a5e8c2b9d7f3a1e6c4b8d2f5a9e7c3b6d4f8a2
✓  Block produced
    Hash:   0x7d9f2a5c8e4b1f3a6c9d5f2a8c1e4b7d3f6a9c2e
    Height: 1
    Txs:    1
```

### Step 6: Check balances

```bash
$ minichain account balance 0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Balance: 999479  # 1000000 - 500 (transfer) - 21 (gas)

$ minichain account balance 0x5c9e2b7d4f1a8c3e6b9d5f2a7c4e1b8d3f6a9c2e
  Balance: 1000500  # 1000000 + 500 (received)
```

### Step 7: Deploy a contract

Create `storage_test.asm`:
```asm
.entry main

main:
    ; Read storage slot 0
    LOADI R0, 0
    SLOAD R1, R0

    ; Increment
    LOADI R2, 1
    ADD R1, R1, R2

    ; Write back
    SSTORE R0, R1
    HALT
```

Deploy:
```bash
$ minichain deploy --from alice --source storage_test.asm

Deploying contract...
  Compiling: storage_test.asm
✓  Compiled to 32 bytes
  Contract Address: 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
✓  Contract deployment submitted
```

Produce a block:
```bash
$ minichain block produce --authority authority_0
✓  Block produced (Height: 2, Txs: 1)
```

Verify deployment:
```bash
$ minichain account info 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7

Account Information:
  Address:      0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
  Balance:      0
  Nonce:        0
  Is Contract:  Yes
  Code Hash:    0x8f2e5a9c7b3d1f6e4a2c8b5d9f7a3e1c6b4d8f2a
```

### Step 8: Call the contract

```bash
$ minichain call --from alice --to 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
✓  Contract call submitted

$ minichain block produce --authority authority_0
✓  Block produced (Height: 3, Txs: 1)

$ minichain call --from bob --to 0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7
✓  Contract call submitted

$ minichain block produce --authority authority_0
✓  Block produced (Height: 4, Txs: 1)
```

The contract's storage slot 0 now holds the value 2 (incremented twice).

### Step 9: Inspect the chain

```bash
$ minichain block list

Recent Blocks:
  #4 3f8c2a6e9b5d1f4a (1 txs)
  #3 5c9e2b7d4f1a8c3e (1 txs)
  #2 a7b3c9e5d1f4a8c2 (1 txs)
  #1 7d9f2a5c8e4b1f3a (1 txs)
  #0 8f2e5a9c7b3d1f6e (0 txs)

$ minichain block info 4

Block Information:
  Height:       4
  Hash:         0x3f8c2a6e9b5d1f4a7c3e8b2d6f9a5c1e4b7d3f8a
  Transactions: 1

Transactions:
  1. e2a5c9b7d4f1a8c3  (call contract)
```

## Best Practices

### Keypair Management

**DO:**
- Use descriptive names for keypairs (`alice`, `treasury`, `authority_0`)
- Back up keypair JSON files securely
- Keep separate keypairs for testing and production

**DON'T:**
- Commit private keys to version control
- Share keypair files over insecure channels
- Reuse keypairs across different chains

### Gas Management

Always estimate gas costs before submitting transactions:

| Operation | Gas Cost |
|-----------|----------|
| Transfer | 21,000 |
| Deploy (per byte) | 200 |
| Call (per byte calldata) | 68 |
| SLOAD | 100 |
| SSTORE | 5,000 |

**Example:** Deploying a 100-byte contract with gas price 2:
```
gas = 21,000 + (100 * 200) = 41,000
cost = 41,000 * 2 = 82,000 units
```

Ensure your account balance is at least `amount + (gas_limit * gas_price)`.

### Block Production

In Proof of Authority:
- Only designated authorities can produce blocks
- Authorities rotate in round-robin order
- Use `block list` to check current height
- Calculate your turn: `height % authority_count`

**Example with 3 authorities:**
```
Height 0 → Authority 0
Height 1 → Authority 1
Height 2 → Authority 2
Height 3 → Authority 0 (wraps around)
```

### Error Handling

Common errors and solutions:

| Error | Solution |
|-------|----------|
| "Insufficient balance" | Check balance with `account balance`, ensure enough for amount + gas |
| "Keypair file not found" | Verify the keypair name (without `.json`) and data directory |
| "Address is not an authority" | Use correct authority keypair for `block produce` |
| "Invalid nonce" | Another transaction is pending; wait for block production |
| "Address is not a contract" | Double-check the contract address; may not be deployed yet |

### Scripting Workflows

The CLI is designed for automation. Example bash script:

```bash
#!/bin/bash
# fund_and_deploy.sh

# Initialize chain
minichain init --authorities 1

# Create accounts
minichain account new --name deployer

# Deploy contract
minichain deploy --from deployer --source contract.asm

# Produce block
minichain block produce --authority authority_0

# Check deployment
CONTRACT_ADDR=$(minichain account list | grep deployer | awk '{print $2}')
minichain account info "$CONTRACT_ADDR"
```

## Implementation Details

### Command Structure (Clap)

Each command is defined using clap's derive macros:

```rust
#[derive(Args)]
pub struct TxArgs {
    #[command(subcommand)]
    command: TxCommand,
}

#[derive(Subcommand)]
enum TxCommand {
    Send {
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,

        #[arg(short, long)]
        from: String,

        #[arg(short, long)]
        to: String,

        #[arg(short, long)]
        amount: u64,
    },
}
```

This automatically generates:
- Help messages (`minichain tx send --help`)
- Type checking (rejects non-numeric amounts)
- Default values
- Short and long option formats

### Colored Output

We use the `colored` crate for terminal formatting:

```rust
use colored::Colorize;

println!("{}  Success", "✓".green().bold());
println!("  Address: {}", address.to_hex().bright_yellow());
println!("  Balance: {}", balance.to_string().bright_cyan());
```

This improves readability:
- Green for success indicators
- Yellow for addresses and hashes
- Cyan for numbers
- Black (dimmed) for metadata

### Helper Functions

To avoid duplication, common operations are extracted:

**Loading keypairs:**
```rust
fn load_keypair(keys_dir: &PathBuf, name: &str) -> Result<Keypair> {
    let key_file = keys_dir.join(format!("{}.json", name));
    let contents = fs::read_to_string(&key_file)?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;
    // ... parse private_key from JSON
}
```

**Loading config:**
```rust
fn load_config(data_dir: &PathBuf) -> Result<BlockchainConfig> {
    let config_file = data_dir.join("config.json");
    let contents = fs::read_to_string(&config_file)?;
    // ... parse authorities, block_time, max_block_size
}
```

**Registering authorities:**
```rust
fn register_authorities(blockchain: &mut Blockchain, data_dir: &PathBuf) -> Result<()> {
    // Scan data/keys/ for authority_*.json files
    // Load public keys
    // Call blockchain.register_authority() for each
}
```

These helpers are used in `tx.rs`, `block.rs`, `deploy.rs`, and `call.rs`.

### Data Directory Structure

```
data/
├── keys/                # Keypair JSON files
│   ├── authority_0.json
│   ├── authority_1.json
│   ├── alice.json
│   └── bob.json
├── config.json          # Blockchain configuration
└── sled/                # Storage database (managed by Storage crate)
    ├── conf
    ├── db
    └── snap.*/
```

**config.json format:**
```json
{
  "authorities": [
    "0xf4a5e8c2b9d7f3a1e6c4b8d2f5a9e7c3b6d4f8a2",
    "0xa7b3c9e5d1f4a8c2b6d9f3e7a5c1b8d4f2a6e9c7"
  ],
  "block_time": 5,
  "max_block_size": 1000
}
```

## Summary

In this chapter, we built a comprehensive CLI that brings all previous components together:

| Component | Integration |
|-----------|-------------|
| Storage ([Ch 2](/minichain/part2/chapter2-storage)) | Load/save accounts, blocks, transactions |
| VM ([Ch 3](/minichain/part3/chapter3-vm)) | Execute contract code during block production |
| Assembler ([Ch 4](/minichain/part4/chapter4-assembler)) | Compile `.asm` files in `deploy` command |
| Blockchain ([Ch 5](/minichain/part5/chapter5-chain)) | Submit txs, produce blocks, validate |

The CLI provides:
- **init**: Bootstrap new chains with genesis blocks
- **account**: Keypair management and balance queries
- **tx send**: Value transfers
- **block**: Query and produce blocks
- **deploy**: Compile and deploy contracts
- **call**: Invoke contract functions

### Design Principles

1. **User-friendly**: Clear help messages, colored output, descriptive errors
2. **Composable**: Commands can be scripted and automated
3. **Safe**: Validates inputs, checks balances, prevents invalid operations
4. **Consistent**: Same patterns across all commands (load keypair, load config, submit)

### Next Steps

You now have a complete, working blockchain implementation! Possible extensions:

- **RPC server**: Build a JSON-RPC API for remote access
- **Web UI**: Create a browser-based block explorer
- **Smart contract language**: Design a high-level language that compiles to assembly
- **Network layer**: Add P2P networking for multi-node deployments
- **Advanced features**: Gas refunds, precompiles, EVM compatibility

The CLI serves as both an end-user tool and a foundation for building higher-level interfaces. Whether you're testing contracts, managing a local chain, or building a distributed system, the CLI provides the essential operations you need.
