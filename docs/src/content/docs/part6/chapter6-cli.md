---
title: "Chapter 6: Command Line Interface"
description: Building a full-featured CLI for minichain
---

# Chapter 6: Command Line Interface

*Coming soon...*

In this chapter, we'll build a complete CLI to interact with our blockchain.

## Commands

```bash
minichain
├── init                    # Initialize new chain
├── account
│   ├── new                 # Generate keypair
│   ├── balance <ADDRESS>   # Query balance
│   └── list                # List accounts
├── tx
│   └── send                # Transfer value
├── deploy                  # Deploy contract
├── call                    # Call contract
├── mine                    # Produce block
├── block
│   ├── get <HASH|HEIGHT>   # Query block
│   └── latest              # Get latest
└── explore                 # Block explorer
```

## Example Session

```bash
$ minichain init --authority 0xABC...
$ minichain account new
$ minichain tx send --from 0x... --to 0x... --value 1000
$ minichain mine --key <authority_key>
$ minichain block latest
```
