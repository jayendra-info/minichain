---
title: "Chapter 5: Consensus & Chain"
description: Implementing Proof of Authority and blockchain orchestration
---

# Chapter 5: Consensus & Chain

*Coming soon...*

In this chapter, we'll bring everything together with:

## Consensus (PoA)

- Authority registration
- Block signature verification
- Validation rules

## Chain Management

- Mempool for pending transactions
- Transaction validation
- Block execution
- State transitions
- Genesis block creation

## The Blockchain Struct

```rust
pub struct Blockchain {
    storage: Storage,
    mempool: Mempool,
    authority: Address,
    head: Hash,
}
```
