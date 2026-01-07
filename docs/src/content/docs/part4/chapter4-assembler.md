---
title: "Chapter 4: Assembly to Bytecode"
description: Building an assembler for our VM
---

# Chapter 4: Assembly to Bytecode

*Coming soon...*

In this chapter, we'll build an assembler that compiles human-readable assembly into VM bytecode.

## Assembly Syntax

```asm
; Counter contract
.entry main

main:
    PUSH r0, 0          ; storage slot 0
    SLOAD r1, r0        ; load current value
    PUSH r2, 1
    ADD r1, r1, r2      ; increment
    SSTORE r0, r1       ; store back
    HALT
```

## What We'll Build

- Lexer (tokenization with logos)
- Parser (AST construction)
- Label resolution
- Bytecode emission
- Error messages with line numbers
