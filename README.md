# Minichain

A minimal blockchain implementation in Rust — built for learning.

## What is this?

A single-node, account-based blockchain featuring:

- **Blake3** hashing
- **Ed25519** signatures
- **Register-based VM** (16 registers, gas metering)
- **Proof of Authority** consensus
- **Custom assembly language**
- **Full CLI**

## Project Structure


Install rust by following https://rust-lang.org/tools/install/



```
minichain/
├── crates/
│   ├── core/        # Primitives: hash, crypto, accounts, blocks, transactions
│   ├── vm/          # Virtual machine
│   ├── storage/     # Persistent state (sled)
│   ├── consensus/   # PoA validation
│   ├── chain/       # Blockchain orchestration
│   ├── assembler/   # Assembly → bytecode
│   └── cli/         # Command-line interface
├── contracts/       # Example .asm contracts
├── docs/            # Astro Starlight documentation
└── tests/           # Integration tests
```

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Run CLI
cargo run -p minichain-cli
```

## Documentation

```bash
cd docs
bun install
bun run dev      # http://localhost:4321
```

## Status

🚧 **Work in Progress**

- [x] Core primitives (hash, crypto, accounts, transactions, blocks, merkle)
- [ ] Storage layer
- [ ] Virtual machine
- [ ] Assembler
- [ ] Consensus & chain
- [ ] CLI

## License

MIT
