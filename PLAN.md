Excellent! Let me generate your complete implementation plan.

---

# Blockchain POC Implementation Plan

## Project Overview

**Name:** `minichain` (or pick your own)

**Goal:** A single-node, account-based blockchain with a register-based VM, Proof of Authority consensus, and persistent storage.

---

## 1. Project Structure

```
minichain/
├── Cargo.toml                 # Workspace root
├── README.md
│
├── crates/
│   ├── core/                  # Blockchain primitives
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── block.rs       # Block, BlockHeader
│   │       ├── transaction.rs # Transaction types
│   │       ├── account.rs     # Account state
│   │       ├── merkle.rs      # Merkle tree
│   │       ├── hash.rs        # Blake3 helpers
│   │       └── crypto.rs      # Ed25519 signing
│   │
│   ├── vm/                    # Register-based VM
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── opcodes.rs     # Instruction definitions
│   │       ├── executor.rs    # VM execution loop
│   │       ├── memory.rs      # VM memory model
│   │       ├── gas.rs         # Gas metering
│   │       └── tracer.rs      # Execution tracer
│   │
│   ├── storage/               # Persistent state
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs          # sled wrapper
│   │       ├── state.rs       # World state (accounts)
│   │       └── chain.rs       # Block storage
│   │
│   ├── consensus/             # PoA logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── poa.rs         # Authority validation
│   │       └── validator.rs   # Block validation rules
│   │
│   ├── chain/                 # Blockchain orchestration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── blockchain.rs  # Main chain logic
│   │       ├── mempool.rs     # Pending transactions
│   │       └── executor.rs    # Block execution
│   │
│   ├── assembler/             # Assembly → bytecode
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lexer.rs       # Tokenization
│   │       ├── parser.rs      # Parse assembly
│   │       └── compiler.rs    # Emit bytecode
│   │
│   └── cli/                   # Command-line interface
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── init.rs    # Initialize chain
│           │   ├── account.rs # Create/query accounts
│           │   ├── tx.rs      # Send transactions
│           │   ├── block.rs   # Mine/query blocks
│           │   ├── deploy.rs  # Deploy contracts
│           │   ├── call.rs    # Call contracts
│           │   └── explore.rs # Block explorer
│           └── repl.rs        # Interactive mode
│
├── contracts/                 # Example contracts (assembly)
│   ├── counter.asm
│   ├── token.asm
│   └── storage_test.asm
│
└── tests/                     # Integration tests
    ├── vm_tests.rs
    ├── chain_tests.rs
    └── e2e_tests.rs
```

---

## 2. Crate Dependency Graph

```
                    ┌─────────────┐
                    │     cli     │
                    └──────┬──────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
    ┌───────────┐   ┌───────────┐   ┌───────────┐
    │ assembler │   │   chain   │   │  explore  │
    └───────────┘   └─────┬─────┘   └───────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
    ┌───────────┐   ┌───────────┐   ┌───────────┐
    │    vm     │   │ consensus │   │  storage  │
    └─────┬─────┘   └─────┬─────┘   └─────┬─────┘
          │               │               │
          └───────────────┼───────────────┘
                          │
                          ▼
                    ┌───────────┐
                    │   core    │
                    └───────────┘
```

---

## 3. Suggested Dependencies

```toml
# Workspace Cargo.toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.dependencies]
# Crypto
blake3 = "1.5"
ed25519-dalek = { version = "2.1", features = ["rand_core"] }
rand = "0.8"

# Storage
sled = "0.34"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# CLI
clap = { version = "4.4", features = ["derive"] }
colored = "2.1"

# Utils
thiserror = "1.0"
anyhow = "1.0"
hex = "0.4"
chrono = { version = "0.4", features = ["serde"] }

# Assembler
logos = "0.14"          # Lexer generator

# Tracing/logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 4. Key Data Structures

### Core Types

```rust
// crates/core/src/hash.rs
use blake3::Hash;

pub type H256 = [u8; 32];

pub fn hash(data: &[u8]) -> H256 {
    blake3::hash(data).into()
}

pub fn merkle_root(hashes: &[H256]) -> H256 {
    // Build tree, return root
}
```

```rust
// crates/core/src/crypto.rs
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};

pub type Address = [u8; 20]; // First 20 bytes of public key hash

pub struct Keypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl Keypair {
    pub fn generate() -> Self { /* ... */ }
    pub fn address(&self) -> Address { /* ... */ }
    pub fn sign(&self, message: &[u8]) -> Signature { /* ... */ }
}
```

```rust
// crates/core/src/account.rs
use crate::{Address, H256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub nonce: u64,
    pub balance: u64,
    pub code_hash: Option<H256>,  // None for EOA, Some for contract
    pub storage_root: H256,
}

impl Account {
    pub fn new_user(balance: u64) -> Self { /* ... */ }
    pub fn new_contract(code_hash: H256) -> Self { /* ... */ }
    pub fn is_contract(&self) -> bool { self.code_hash.is_some() }
}
```

```rust
// crates/core/src/transaction.rs
use crate::{Address, H256, Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub nonce: u64,
    pub from: Address,
    pub to: Option<Address>,      // None = contract deployment
    pub value: u64,
    pub data: Vec<u8>,            // Calldata or bytecode
    pub gas_limit: u64,
    pub gas_price: u64,
    pub signature: Signature,
}

impl Transaction {
    pub fn hash(&self) -> H256 { /* ... */ }
    pub fn verify(&self) -> bool { /* ... */ }
    pub fn is_deploy(&self) -> bool { self.to.is_none() }
}
```

```rust
// crates/core/src/block.rs
use crate::{H256, Address, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub prev_hash: H256,
    pub merkle_root: H256,
    pub state_root: H256,
    pub author: Address,          // PoA authority
    pub difficulty: u64,          // Kept for structure, always 1 for PoA
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub signature: Signature,     // Authority signature
}

impl Block {
    pub fn hash(&self) -> H256 { /* hash header */ }
    pub fn genesis(authority: Address) -> Self { /* ... */ }
}
```

### VM Types

```rust
// crates/vm/src/opcodes.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Arithmetic (0x00 - 0x0F)
    ADD   = 0x01,  // r[a] = r[b] + r[c]
    SUB   = 0x02,  // r[a] = r[b] - r[c]
    MUL   = 0x03,  // r[a] = r[b] * r[c]
    DIV   = 0x04,  // r[a] = r[b] / r[c]
    MOD   = 0x05,  // r[a] = r[b] % r[c]
    
    // Logic (0x10 - 0x1F)
    AND   = 0x10,  // r[a] = r[b] & r[c]
    OR    = 0x11,  // r[a] = r[b] | r[c]
    XOR   = 0x12,  // r[a] = r[b] ^ r[c]
    NOT   = 0x13,  // r[a] = !r[b]
    SHL   = 0x14,  // r[a] = r[b] << r[c]
    SHR   = 0x15,  // r[a] = r[b] >> r[c]
    
    // Comparison (0x20 - 0x2F)
    EQ    = 0x20,  // r[a] = r[b] == r[c]
    LT    = 0x21,  // r[a] = r[b] < r[c]
    GT    = 0x22,  // r[a] = r[b] > r[c]
    ISZERO= 0x23,  // r[a] = r[b] == 0
    
    // Memory (0x30 - 0x3F)
    LOAD  = 0x30,  // r[a] = memory[r[b]]
    STORE = 0x31,  // memory[r[a]] = r[b]
    MSIZE = 0x32,  // r[a] = memory.size()
    
    // Storage (0x40 - 0x4F)
    SLOAD = 0x40,  // r[a] = storage[r[b]]
    SSTORE= 0x41,  // storage[r[a]] = r[b]
    
    // Control Flow (0x50 - 0x5F)
    JUMP  = 0x50,  // pc = r[a]
    JUMPI = 0x51,  // if r[b] != 0: pc = r[a]
    HALT  = 0x52,  // stop execution (success)
    REVERT= 0x53,  // stop execution (failure)
    RETURN= 0x54,  // return memory[r[a]..r[a]+r[b]]
    
    // Context (0x60 - 0x6F)
    CALLER    = 0x60,  // r[a] = msg.sender
    CALLVALUE = 0x61,  // r[a] = msg.value
    BALANCE   = 0x62,  // r[a] = balance(r[b])
    BLOCKHASH = 0x63,  // r[a] = blockhash
    TIMESTAMP = 0x64,  // r[a] = block.timestamp
    NUMBER    = 0x65,  // r[a] = block.number
    
    // Crypto (0x70 - 0x7F)
    HASH  = 0x70,  // r[a] = blake3(memory[r[b]..r[b]+r[c]])
    
    // Data (0x80 - 0x8F)
    PUSH  = 0x80,  // r[a] = immediate (next 8 bytes)
    MOV   = 0x81,  // r[a] = r[b]
}
```

```rust
// crates/vm/src/gas.rs

pub struct GasTable {
    pub base: u64,
}

impl Opcode {
    pub fn gas_cost(&self) -> u64 {
        match self {
            // Cheap operations
            Opcode::ADD | Opcode::SUB | Opcode::MOV |
            Opcode::AND | Opcode::OR | Opcode::XOR |
            Opcode::NOT | Opcode::EQ | Opcode::LT |
            Opcode::GT | Opcode::ISZERO | Opcode::PUSH => 3,
            
            // Medium operations
            Opcode::MUL | Opcode::DIV | Opcode::MOD |
            Opcode::SHL | Opcode::SHR => 5,
            
            // Memory operations
            Opcode::LOAD | Opcode::STORE | Opcode::MSIZE => 5,
            
            // Storage (expensive!)
            Opcode::SLOAD => 200,
            Opcode::SSTORE => 5000,  // Much higher for writes
            
            // Control flow
            Opcode::JUMP | Opcode::JUMPI => 8,
            Opcode::HALT | Opcode::REVERT | Opcode::RETURN => 0,
            
            // Context
            Opcode::CALLER | Opcode::CALLVALUE |
            Opcode::BLOCKHASH | Opcode::TIMESTAMP |
            Opcode::NUMBER => 2,
            Opcode::BALANCE => 100,
            
            // Crypto
            Opcode::HASH => 30,
        }
    }
}
```

```rust
// crates/vm/src/executor.rs

pub const NUM_REGISTERS: usize = 16;

pub struct VM {
    pub registers: [u64; NUM_REGISTERS],
    pub memory: Vec<u8>,
    pub pc: usize,                    // Program counter
    pub gas_remaining: u64,
    pub gas_used: u64,
    pub halted: bool,
    pub reverted: bool,
    pub return_data: Vec<u8>,
}

pub struct ExecutionContext<'a> {
    pub caller: Address,
    pub address: Address,             // Current contract
    pub value: u64,
    pub data: &'a [u8],               // Calldata
    pub code: &'a [u8],               // Bytecode
    pub storage: &'a mut dyn Storage,
    pub block: &'a BlockHeader,
}

#[derive(Debug)]
pub enum ExecutionResult {
    Success { gas_used: u64, return_data: Vec<u8> },
    Revert { gas_used: u64 },
    OutOfGas,
    InvalidOpcode(u8),
    InvalidJump,
}

impl VM {
    pub fn new(gas_limit: u64) -> Self { /* ... */ }
    
    pub fn execute(&mut self, ctx: &mut ExecutionContext) -> ExecutionResult {
        // Main execution loop
    }
}
```

---

## 5. Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Goal:** Core types compile, basic hashing and signing works.

| Task | Crate | Deliverable |
|------|-------|-------------|
| 1.1 | `core` | Hash type, Blake3 helpers |
| 1.2 | `core` | Ed25519 keypair, signing, verification |
| 1.3 | `core` | Address derivation from public key |
| 1.4 | `core` | Account struct |
| 1.5 | `core` | Transaction struct with signing |
| 1.6 | `core` | Block and BlockHeader structs |
| 1.7 | `core` | Merkle tree implementation |
| 1.8 | `core` | Unit tests for all above |

**Milestone:** Can create keypairs, sign transactions, build blocks in memory.

---

### Phase 2: Storage Layer (Week 2-3)

**Goal:** Persistent storage for accounts, blocks, and contract storage.

| Task | Crate | Deliverable |
|------|-------|-------------|
| 2.1 | `storage` | sled database wrapper |
| 2.2 | `storage` | Account state storage (CRUD) |
| 2.3 | `storage` | Block storage (by hash, by height) |
| 2.4 | `storage` | Contract code storage |
| 2.5 | `storage` | Contract storage (key-value per contract) |
| 2.6 | `storage` | State root calculation |
| 2.7 | `storage` | Unit tests |

**Milestone:** Can persist and retrieve accounts, blocks, contract state.

---

### Phase 3: Virtual Machine (Week 3-5)

**Goal:** Working register-based VM with gas metering.

| Task | Crate | Deliverable |
|------|-------|-------------|
| 3.1 | `vm` | Opcode enum and encoding |
| 3.2 | `vm` | VM struct (registers, memory, pc) |
| 3.3 | `vm` | Instruction decoding |
| 3.4 | `vm` | Arithmetic opcodes |
| 3.5 | `vm` | Logic opcodes |
| 3.6 | `vm` | Comparison opcodes |
| 3.7 | `vm` | Memory opcodes |
| 3.8 | `vm` | Control flow opcodes |
| 3.9 | `vm` | Storage opcodes (SLOAD/SSTORE) |
| 3.10 | `vm` | Context opcodes (CALLER, etc.) |
| 3.11 | `vm` | HASH opcode |
| 3.12 | `vm` | Gas metering |
| 3.13 | `vm` | Execution tracer |
| 3.14 | `vm` | Unit tests (one per opcode group) |

**Milestone:** Can execute bytecode, trace execution, measure gas.

---

### Phase 4: Assembler (Week 5-6)

**Goal:** Write human-readable assembly, compile to bytecode.

| Task | Crate | Deliverable |
|------|-------|-------------|
| 4.1 | `assembler` | Assembly syntax design |
| 4.2 | `assembler` | Lexer (tokenization) |
| 4.3 | `assembler` | Parser (AST) |
| 4.4 | `assembler` | Label resolution |
| 4.5 | `assembler` | Bytecode emission |
| 4.6 | `assembler` | Error messages with line numbers |
| 4.7 | `assembler` | Unit tests |

**Assembly Syntax Example:**
```asm
; Counter contract
; Stores a counter, allows increment

.entry main

main:
    PUSH r0, 0              ; storage slot 0
    SLOAD r1, r0            ; load current value
    PUSH r2, 1
    ADD r1, r1, r2          ; increment
    SSTORE r0, r1           ; store back
    HALT

; Entry point for "get" function
get:
    PUSH r0, 0
    SLOAD r1, r0
    ; Return r1 somehow (need RETURN setup)
    HALT
```

**Milestone:** Can compile `.asm` files to bytecode.

---

### Phase 5: Consensus & Chain (Week 6-7)

**Goal:** PoA consensus, block production and validation.

| Task | Crate | Deliverable |
|------|-------|-------------|
| 5.1 | `consensus` | Authority registry |
| 5.2 | `consensus` | Block signature verification |
| 5.3 | `consensus` | Block validation rules |
| 5.4 | `chain` | Mempool (pending transactions) |
| 5.5 | `chain` | Transaction validation |
| 5.6 | `chain` | Block executor (apply txs to state) |
| 5.7 | `chain` | Contract deployment execution |
| 5.8 | `chain` | Contract call execution |
| 5.9 | `chain` | Blockchain struct (head, fork choice) |
| 5.10 | `chain` | Genesis block creation |
| 5.11 | `chain` | Integration tests |

**Milestone:** Can produce blocks, execute transactions, update state.

---

### Phase 6: CLI (Week 7-8)

**Goal:** Full command-line interface.

| Task | Crate | Deliverable |
|------|-------|-------------|
| 6.1 | `cli` | CLI framework setup (clap) |
| 6.2 | `cli` | `init` — create genesis, initialize DB |
| 6.3 | `cli` | `account new` — generate keypair |
| 6.4 | `cli` | `account balance` — query balance |
| 6.5 | `cli` | `tx send` — transfer value |
| 6.6 | `cli` | `deploy` — deploy contract from .asm |
| 6.7 | `cli` | `call` — call contract method |
| 6.8 | `cli` | `mine` — produce a block |
| 6.9 | `cli` | `block get` — query block by hash/height |
| 6.10 | `cli` | `explore` — interactive block explorer |
| 6.11 | `cli` | Pretty output with colors |

**Milestone:** Fully usable blockchain via CLI.

---

### Phase 7: Polish & Extras (Week 8+)

| Task | Deliverable |
|------|-------------|
| 7.1 | Example contracts (counter, token, storage test) |
| 7.2 | Block explorer improvements |
| 7.3 | VM tracer output formatting |
| 7.4 | Documentation |
| 7.5 | README with tutorial |

---

## 6. CLI Commands Overview

```
minichain
├── init                        # Initialize new chain
│   └── --authority <ADDRESS>   # Set PoA authority
│
├── account
│   ├── new                     # Generate new keypair
│   ├── list                    # List known accounts
│   ├── balance <ADDRESS>       # Query balance
│   └── nonce <ADDRESS>         # Query nonce
│
├── tx
│   ├── send                    # Send value transfer
│   │   ├── --from <ADDRESS>
│   │   ├── --to <ADDRESS>
│   │   ├── --value <AMOUNT>
│   │   └── --key <PRIVATE_KEY>
│   └── status <TX_HASH>        # Check tx status
│
├── deploy                      # Deploy contract
│   ├── --file <FILE.asm>       # Assembly file
│   ├── --from <ADDRESS>
│   ├── --key <PRIVATE_KEY>
│   └── --gas <LIMIT>
│
├── call                        # Call contract
│   ├── --to <CONTRACT>
│   ├── --from <ADDRESS>
│   ├── --data <HEX>            # Calldata
│   ├── --value <AMOUNT>
│   └── --trace                 # Enable VM tracer
│
├── mine                        # Produce block
│   └── --key <AUTHORITY_KEY>
│
├── block
│   ├── get <HASH|HEIGHT>       # Query block
│   └── latest                  # Get latest block
│
└── explore                     # Interactive block explorer
    ├── blocks                  # List recent blocks
    ├── txs                     # List recent transactions  
    └── contract <ADDRESS>      # Inspect contract
```

---

## 7. Example Session

```bash
# Initialize chain
$ minichain init --authority 0xABC...

# Create accounts
$ minichain account new
Generated keypair:
  Address: 0x1234...
  Private: 0xABCD... (save this!)

$ minichain account new
Generated keypair:
  Address: 0x5678...
  Private: 0xEF01...

# Fund account (authority can mint in genesis)
$ minichain tx send --from 0xABC... --to 0x1234... --value 1000000 --key <auth_key>

# Mine block
$ minichain mine --key <auth_key>
Block #1 mined: 0xBEEF...
  Transactions: 1
  Gas used: 21000

# Check balance
$ minichain account balance 0x1234...
Balance: 1000000

# Deploy counter contract
$ minichain deploy --file contracts/counter.asm --from 0x1234... --key <key> --gas 100000
Contract deployed at: 0xCONTRACT...

# Call with trace
$ minichain call --to 0xCONTRACT... --from 0x1234... --trace
VM Trace:
  [0000] PUSH r0, 0          | r0=0 | gas=99997
  [0009] SLOAD r1, r0        | r1=0 | gas=99797
  [0012] PUSH r2, 1          | r2=1 | gas=99794
  [0021] ADD r1, r1, r2      | r1=1 | gas=99791
  [0024] SSTORE r0, r1       |      | gas=94791
  [0027] HALT                |      | gas=94791
Result: Success
Gas used: 5209

# Explore
$ minichain explore blocks
Height  Hash        Txs  Author      Time
------  ----------  ---  ----------  ----
1       0xBEEF...   1    0xABC...    2 min ago
0       0x0000...   0    0xABC...    5 min ago
```

---

## 8. Next Steps

Ready to start coding? Here's what to do:

1. **Create the workspace:**
   ```bash
   cargo new minichain
   cd minichain
   # Set up workspace structure
   ```

2. **Start with Phase 1** — get `core` crate compiling with basic types

3. **Write tests as you go** — each module should have unit tests

---

Want me to generate any of these in detail?

- **Starter code** for any specific crate
- **Full opcode encoding/decoding** implementation
- **Assembly grammar** specification
- **Test cases** for VM opcodes
- **Detailed Merkle tree** implementation