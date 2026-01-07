---
title: "Chapter 3: Register-Based VM"
description: Building the virtual machine for smart contract execution
---

# Chapter 3: Register-Based VM

*Coming soon...*

In this chapter, we'll build a register-based virtual machine featuring:

- 16 general-purpose registers
- Stack and memory operations
- Arithmetic and logic opcodes
- Storage access (SLOAD/SSTORE)
- Gas metering
- Execution tracing

## Opcode Categories

| Range | Category |
|-------|----------|
| 0x00-0x0F | Arithmetic |
| 0x10-0x1F | Logic |
| 0x20-0x2F | Comparison |
| 0x30-0x3F | Memory |
| 0x40-0x4F | Storage |
| 0x50-0x5F | Control Flow |
| 0x60-0x6F | Context |
| 0x70-0x7F | Crypto |
| 0x80-0x8F | Data |
