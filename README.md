# Minichain

A minimal toy blockchain implementation in Rust — built for learning.

## What is this?

A complete, single-node, account-based blockchain featuring:

- **Blake3** hashing for cryptographic operations
- **Ed25519** signatures for transaction signing
- **Register-based VM** with 16 registers and gas metering
- **Proof of Authority** consensus with round-robin block production
- **Custom assembly language** with human-readable syntax
- **Full-featured CLI** for chain interaction
- **Persistent storage** using sled embedded database
- **Merkle trees** for efficient state verification

Built as an educational project to understand blockchain internals from scratch.

## Features

### Core Components

- **Accounts & Balances**: Account-based model with nonce tracking
- **Transactions**: Transfer, deploy, and call transaction types
- **Gas System**: Ethereum-inspired gas metering (storage ops ~100x arithmetic)
- **Smart Contracts**: Deploy and execute bytecode contracts
- **Block Production**: PoA consensus with designated authorities
- **Mempool**: Transaction pool with gas-price ordering
- **State Management**: Persistent state with atomic updates

### Virtual Machine

- 16 general-purpose registers (R0-R15)
- 60+ opcodes covering arithmetic, logic, memory, storage, and control flow
- Separate memory (RAM) and storage (disk) operations
- Gas metering on every operation
- Stack-based function calls with CALL/RET

### Assembly Language

- Human-readable assembly syntax
- Labels and jump instructions
- Immediate values and register operations
- Entry point declarations (`.entry`)
- Compiles to VM bytecode

## Project Structure

```
minichain/
├── crates/
│   ├── core/        # Primitives: hash, crypto, accounts, blocks, transactions
│   ├── storage/     # Persistent state layer (sled)
│   ├── vm/          # Register-based virtual machine with gas metering
│   ├── assembler/   # Assembly language → bytecode compiler
│   ├── consensus/   # Proof of Authority validation and block proposing
│   ├── chain/       # Blockchain orchestration (mempool, executor, validation)
│   └── cli/         # Command-line interface
├── docs/            # Astro Starlight documentation (6 chapters)
└── tests/           # Integration tests
```

## Installation

### Prerequisites

Install Rust from [rust-lang.org/tools/install](https://rust-lang.org/tools/install/)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/minichain.git
cd minichain

# Build the project
cargo build --release

# The CLI binary will be at target/release/minichain
```

## Quick Start

### 1. Initialize a Blockchain

```bash
# Create a new blockchain with 1 authority
cargo run --release -- init --authorities 1

# Output:
# Initializing minichain...
#
# Generating authorities...
#   Authority 1: 0xf4a5e8c2b9d7f3a1...
#
# ✓  Created genesis block
# ✓  Saved authority 1 keypair to: data/keys/authority_0.json
# ✓  Saved config to: data/config.json
```

### 2. Create Accounts

```bash
# Generate keypairs for Alice and Bob
cargo run --release -- account new --name alice
cargo run --release -- account new --name bob

# List all accounts
cargo run --release -- account list
```

### 3. Deploy a Contract

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
cargo run --release -- deploy --from alice --source counter.asm

# Output:
# Deploying contract...
#   Compiling: counter.asm
# ✓  Compiled to 28 bytes
# ✓  Contract deployment submitted
#   Contract Address: 0xa7b3c9e5d1f4a8c2...
```

### 4. Produce a Block

```bash
cargo run --release -- block produce --authority authority_0

# Output:
# Producing new block...
#   Authority: 0xf4a5e8c2b9d7f3a1...
# ✓  Block produced
#     Hash:   0x7d9f2a5c8e4b1f3a...
#     Height: 1
#     Txs:    1
```

### 5. Call the Contract

```bash
cargo run --release -- call --from alice --to 0xa7b3c9e5d1f4a8c2...

# Produce another block
cargo run --release -- block produce --authority authority_0
```

### 6. Explore the Chain

```bash
# List recent blocks
cargo run --release -- block list

# View block details
cargo run --release -- block info 1

# Check account balance
cargo run --release -- account balance 0x3f8c2a6e9b5d1f4a...
```

## CLI Commands

| Command | Description | Example |
|---------|-------------|---------|
| `init` | Initialize new blockchain | `minichain init --authorities 2` |
| `account new` | Generate keypair | `minichain account new --name alice` |
| `account balance` | Query balance | `minichain account balance 0xABC...` |
| `account info` | Show account details | `minichain account info 0xABC...` |
| `account list` | List all keypairs | `minichain account list` |
| `tx send` | Send transfer | `minichain tx send --from alice --to 0xABC... --amount 100` |
| `block list` | List recent blocks | `minichain block list --count 10` |
| `block info` | Show block details | `minichain block info 5` |
| `block produce` | Produce new block | `minichain block produce --authority authority_0` |
| `deploy` | Deploy contract | `minichain deploy --from alice --source contract.asm` |
| `call` | Call contract | `minichain call --from alice --to 0xABC... --data 00` |

Run `minichain --help` or `minichain <command> --help` for detailed usage.

## Development

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test -p minichain-core
cargo test -p minichain-vm
cargo test -p minichain-chain

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Check for compilation errors
cargo check

# Run clippy linter
cargo clippy --all-targets --all-features

# Format code
cargo fmt
```

### Building Documentation

The project includes comprehensive documentation built with Astro Starlight.

#### Local Development

```bash
cd docs
bun install
bun run dev      # Visit http://localhost:4321
```

#### Docker

```bash
cd docs
docker build -t minichain-docs .
docker run -p 8080:8080 minichain-docs
```

Visit http://localhost:8080 to view the documentation.

## Documentation

The documentation covers the implementation in 6 comprehensive chapters:

1. **Chapter 1: Introduction** - Overview and core primitives
2. **Chapter 2: Storage** - Persistent state management with sled
3. **Chapter 3: Virtual Machine** - Register-based VM with gas metering
4. **Chapter 4: Assembler** - Assembly language to bytecode compiler
5. **Chapter 5: Consensus & Chain** - PoA consensus and blockchain orchestration
6. **Chapter 6: CLI** - Command-line interface usage guide

Each chapter includes:
- Conceptual explanations with analogies
- Implementation details and design decisions
- Code examples and usage patterns
- Best practices and optimization tips

## Example: Complete Workflow

Here's a complete end-to-end example:

```bash
# 1. Initialize blockchain
minichain init --authorities 1

# 2. Create accounts
minichain account new --name alice
minichain account new --name bob

# 3. Create a storage test contract (storage_test.asm)
# .entry main
# main:
#     LOADI R0, 0
#     SLOAD R1, R0
#     LOADI R2, 1
#     ADD R1, R1, R2
#     SSTORE R0, R1
#     HALT

# 4. Deploy contract
minichain deploy --from alice --source storage_test.asm

# 5. Produce block to include deployment
minichain block produce --authority authority_0

# 6. Call contract twice
minichain call --from alice --to 0xa7b3c9e5d1f4a8c2...
minichain block produce --authority authority_0

minichain call --from bob --to 0xa7b3c9e5d1f4a8c2...
minichain block produce --authority authority_0

# 7. View blockchain state
minichain block list
minichain block info 3
minichain account info 0xa7b3c9e5d1f4a8c2...
```

The contract's storage slot 0 now holds the value 2 (incremented twice).

## Architecture

### Transaction Flow

```
User → CLI → Blockchain → Mempool → Block Production → Executor → VM → Storage
```

1. **CLI**: User creates and signs transaction
2. **Blockchain**: Validates and submits to mempool
3. **Mempool**: Orders transactions by gas price
4. **Block Production**: Authority collects transactions
5. **Executor**: Executes transactions in VM
6. **VM**: Runs bytecode with gas metering
7. **Storage**: Persists state changes atomically

### Block Production (PoA)

- Round-robin scheduling: `height % authority_count`
- Only designated authority can produce each block
- Authorities sign blocks with Ed25519
- Timestamp validation with configurable clock drift
- Automatic transaction inclusion from mempool

## Gas Costs

| Operation | Gas Cost | Notes |
|-----------|----------|-------|
| Transfer | 21,000 | Base transaction cost |
| SLOAD | 100 | Storage read |
| SSTORE | 5,000 | Storage write |
| LOAD64 | 3 | Memory read |
| STORE64 | 3 | Memory write |
| ADD/SUB/MUL | 2 | Arithmetic operations |
| DIV/MOD | 4 | Division operations |
| CALL | 100 | Call overhead |
| Deploy (per byte) | 200 | Contract deployment |

Storage operations are intentionally expensive to discourage abuse.

## Testing

The project includes 26+ unit and integration tests:

- **Core**: Hashing, signatures, addresses, transactions, blocks, merkle trees
- **Storage**: Account state, chain state, persistence
- **VM**: All opcodes, gas metering, contract execution
- **Assembler**: Lexer, parser, compiler, full assembly programs
- **Consensus**: PoA validation, block proposing, timestamp checks
- **Chain**: Mempool, executor, blockchain operations

Run tests with:
```bash
cargo test --all
```

## Limitations & Future Work

Current limitations (by design for simplicity):
- Single-node only (no P2P networking)
- No transaction/receipt indices
- Basic mempool (no replacement/priority queues)
- No precompiled contracts
- No EVM compatibility

Possible extensions:
- **Networking**: Add P2P layer for multi-node deployment
- **RPC Server**: JSON-RPC API for remote access
- **Web UI**: Browser-based block explorer
- **Smart Contract Language**: High-level language compiling to assembly
- **Advanced Features**: Gas refunds, precompiles, state pruning
- **EVM Compatibility**: Support for Solidity contracts

## Status

✅ **Complete Implementation**

- [x] Core primitives (hash, crypto, accounts, transactions, blocks, merkle)
- [x] Storage layer (persistent state with sled)
- [x] Virtual machine (register-based VM with gas metering)
- [x] Assembler (assembly → bytecode compiler)
- [x] Consensus & chain (PoA, mempool, executor, validation)
- [x] CLI (complete command-line interface)
- [x] Documentation (6 comprehensive chapters)
- [x] Tests (26+ passing tests)
- [x] Code quality (0 clippy warnings)

## Contributing

This is an educational project. Contributions are welcome! Areas to improve:

- Additional test coverage
- Performance optimizations
- Documentation improvements
- Example contracts
- Tooling (debugger, profiler)

## License

MIT

## Acknowledgments

Built as a learning project to understand blockchain internals. Inspired by Ethereum's design but simplified for educational purposes.
