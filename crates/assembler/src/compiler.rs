//! Emit bytecode from AST.
//!
//! Two-pass compilation:
//! 1. First pass: collect label addresses
//! 2. Second pass: emit bytecode with resolved labels

use crate::parser::{Directive, Instruction, Program, Statement};
use std::collections::HashMap;
use thiserror::Error;

/// Compiler errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CompileError {
    #[error("undefined label: {0}")]
    UndefinedLabel(String),

    #[error("duplicate label '{label}' (first defined at address {first_addr})")]
    DuplicateLabel { label: String, first_addr: u64 },

    #[error("invalid register: {0}")]
    InvalidRegister(u8),
}

pub type Result<T> = std::result::Result<T, CompileError>;

/// Opcode constants (matching VM implementation)
mod opcodes {
    // Control flow (0x00-0x0F)
    pub const HALT: u8 = 0x00;
    pub const NOP: u8 = 0x01;
    pub const JUMP: u8 = 0x02;
    pub const JUMPI: u8 = 0x03;
    pub const CALL: u8 = 0x04;
    pub const RET: u8 = 0x05;
    pub const REVERT: u8 = 0x0F;

    // Arithmetic (0x10-0x1F)
    pub const ADD: u8 = 0x10;
    pub const SUB: u8 = 0x11;
    pub const MUL: u8 = 0x12;
    pub const DIV: u8 = 0x13;
    pub const MOD: u8 = 0x14;
    pub const ADDI: u8 = 0x15;

    // Bitwise (0x20-0x2F)
    pub const AND: u8 = 0x20;
    pub const OR: u8 = 0x21;
    pub const XOR: u8 = 0x22;
    pub const NOT: u8 = 0x23;
    pub const SHL: u8 = 0x24;
    pub const SHR: u8 = 0x25;

    // Comparison (0x30-0x3F)
    pub const EQ: u8 = 0x30;
    pub const NE: u8 = 0x31;
    pub const LT: u8 = 0x32;
    pub const GT: u8 = 0x33;
    pub const LE: u8 = 0x34;
    pub const GE: u8 = 0x35;
    pub const ISZERO: u8 = 0x36;

    // Memory (0x40-0x4F)
    pub const LOAD8: u8 = 0x40;
    pub const LOAD64: u8 = 0x41;
    pub const STORE8: u8 = 0x42;
    pub const STORE64: u8 = 0x43;
    pub const MSIZE: u8 = 0x44;
    pub const MCOPY: u8 = 0x45;

    // Storage (0x50-0x5F)
    pub const SLOAD: u8 = 0x50;
    pub const SSTORE: u8 = 0x51;

    // Immediate (0x70-0x7F)
    pub const LOADI: u8 = 0x70;
    pub const MOV: u8 = 0x71;

    // Context (0x80-0x8F)
    pub const CALLER: u8 = 0x80;
    pub const CALLVALUE: u8 = 0x81;
    pub const ADDRESS: u8 = 0x82;
    pub const BLOCKNUMBER: u8 = 0x83;
    pub const TIMESTAMP: u8 = 0x84;
    pub const GAS: u8 = 0x85;

    // Debug (0xF0-0xFF)
    pub const LOG: u8 = 0xF0;
}

/// Compiler state
pub struct Compiler {
    /// Symbol table: label name → address
    symbol_table: HashMap<String, u64>,
    /// Constants: name → value
    constants: HashMap<String, u64>,
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    /// Compile a program to bytecode
    pub fn compile(program: &Program) -> Result<Vec<u8>> {
        let mut compiler = Self::new();

        // First pass: collect labels and constants
        compiler.first_pass(program)?;

        // Second pass: emit bytecode
        compiler.second_pass(program)
    }

    /// First pass: build symbol table
    fn first_pass(&mut self, program: &Program) -> Result<()> {
        let mut address: u64 = 0;

        for statement in &program.statements {
            match statement {
                Statement::Label(name) => {
                    // Check for duplicate labels
                    if let Some(&first_addr) = self.symbol_table.get(name) {
                        return Err(CompileError::DuplicateLabel {
                            label: name.clone(),
                            first_addr,
                        });
                    }
                    self.symbol_table.insert(name.clone(), address);
                }
                Statement::Instruction(inst) => {
                    address += inst.byte_size() as u64;
                }
                Statement::Directive(Directive::Const(name, value)) => {
                    self.constants.insert(name.clone(), *value);
                }
                Statement::Directive(_) => {
                    // Other directives don't emit code
                }
            }
        }

        Ok(())
    }

    /// Second pass: emit bytecode
    fn second_pass(&mut self, program: &Program) -> Result<Vec<u8>> {
        let mut bytecode = Vec::new();

        for statement in &program.statements {
            match statement {
                Statement::Label(_) => {
                    // Labels don't emit bytecode
                }
                Statement::Instruction(inst) => {
                    self.emit_instruction(inst, &mut bytecode)?;
                }
                Statement::Directive(_) => {
                    // Directives don't emit bytecode
                }
            }
        }

        Ok(bytecode)
    }

    /// Emit a single instruction
    fn emit_instruction(&self, inst: &Instruction, bytecode: &mut Vec<u8>) -> Result<()> {
        match inst {
            // No operands
            Instruction::Halt => bytecode.push(opcodes::HALT),
            Instruction::Nop => bytecode.push(opcodes::NOP),
            Instruction::Ret => bytecode.push(opcodes::RET),
            Instruction::Revert => bytecode.push(opcodes::REVERT),

            // Single register
            Instruction::Jump { target } => {
                bytecode.push(opcodes::JUMP);
                bytecode.push(target << 4);
            }
            Instruction::Call { target } => {
                bytecode.push(opcodes::CALL);
                bytecode.push(target << 4);
            }
            Instruction::Log { src } => {
                bytecode.push(opcodes::LOG);
                bytecode.push(src << 4);
            }
            Instruction::MSize { dst } => {
                bytecode.push(opcodes::MSIZE);
                bytecode.push(dst << 4);
            }
            Instruction::Caller { dst } => {
                bytecode.push(opcodes::CALLER);
                bytecode.push(dst << 4);
            }
            Instruction::CallValue { dst } => {
                bytecode.push(opcodes::CALLVALUE);
                bytecode.push(dst << 4);
            }
            Instruction::Address { dst } => {
                bytecode.push(opcodes::ADDRESS);
                bytecode.push(dst << 4);
            }
            Instruction::BlockNumber { dst } => {
                bytecode.push(opcodes::BLOCKNUMBER);
                bytecode.push(dst << 4);
            }
            Instruction::Timestamp { dst } => {
                bytecode.push(opcodes::TIMESTAMP);
                bytecode.push(dst << 4);
            }
            Instruction::Gas { dst } => {
                bytecode.push(opcodes::GAS);
                bytecode.push(dst << 4);
            }

            // Two registers
            Instruction::JumpI { cond, target } => {
                bytecode.push(opcodes::JUMPI);
                bytecode.push((cond << 4) | target);
            }
            Instruction::Mov { dst, src } => {
                bytecode.push(opcodes::MOV);
                bytecode.push((dst << 4) | src);
            }
            Instruction::Not { dst, src } => {
                bytecode.push(opcodes::NOT);
                bytecode.push((dst << 4) | src);
            }
            Instruction::IsZero { dst, src } => {
                bytecode.push(opcodes::ISZERO);
                bytecode.push((dst << 4) | src);
            }
            Instruction::Load8 { dst, addr } => {
                bytecode.push(opcodes::LOAD8);
                bytecode.push((dst << 4) | addr);
            }
            Instruction::Load64 { dst, addr } => {
                bytecode.push(opcodes::LOAD64);
                bytecode.push((dst << 4) | addr);
            }
            Instruction::Store8 { addr, src } => {
                bytecode.push(opcodes::STORE8);
                bytecode.push((addr << 4) | src);
            }
            Instruction::Store64 { addr, src } => {
                bytecode.push(opcodes::STORE64);
                bytecode.push((addr << 4) | src);
            }
            Instruction::SLoad { dst, key } => {
                bytecode.push(opcodes::SLOAD);
                bytecode.push((dst << 4) | key);
            }
            Instruction::SStore { key, value } => {
                bytecode.push(opcodes::SSTORE);
                bytecode.push((key << 4) | value);
            }

            // Three registers
            Instruction::Add { dst, s1, s2 } => {
                bytecode.push(opcodes::ADD);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Sub { dst, s1, s2 } => {
                bytecode.push(opcodes::SUB);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Mul { dst, s1, s2 } => {
                bytecode.push(opcodes::MUL);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Div { dst, s1, s2 } => {
                bytecode.push(opcodes::DIV);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Mod { dst, s1, s2 } => {
                bytecode.push(opcodes::MOD);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::And { dst, s1, s2 } => {
                bytecode.push(opcodes::AND);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Or { dst, s1, s2 } => {
                bytecode.push(opcodes::OR);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Xor { dst, s1, s2 } => {
                bytecode.push(opcodes::XOR);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Shl { dst, s1, s2 } => {
                bytecode.push(opcodes::SHL);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Shr { dst, s1, s2 } => {
                bytecode.push(opcodes::SHR);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Eq { dst, s1, s2 } => {
                bytecode.push(opcodes::EQ);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Ne { dst, s1, s2 } => {
                bytecode.push(opcodes::NE);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Lt { dst, s1, s2 } => {
                bytecode.push(opcodes::LT);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Gt { dst, s1, s2 } => {
                bytecode.push(opcodes::GT);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Le { dst, s1, s2 } => {
                bytecode.push(opcodes::LE);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::Ge { dst, s1, s2 } => {
                bytecode.push(opcodes::GE);
                bytecode.push((dst << 4) | s1);
                bytecode.push(s2 << 4);
            }
            Instruction::MCopy { dst, src, len } => {
                bytecode.push(opcodes::MCOPY);
                bytecode.push((dst << 4) | src);
                bytecode.push(len << 4);
            }

            // Register + immediate
            Instruction::LoadI { dst, value } => {
                bytecode.push(opcodes::LOADI);
                bytecode.push(dst << 4);
                bytecode.extend_from_slice(&value.to_le_bytes());
            }
            Instruction::LoadILabel { dst, label } => {
                // Resolve label to address
                let addr = self
                    .symbol_table
                    .get(label)
                    .or_else(|| self.constants.get(label))
                    .ok_or_else(|| CompileError::UndefinedLabel(label.clone()))?;

                bytecode.push(opcodes::LOADI);
                bytecode.push(dst << 4);
                bytecode.extend_from_slice(&addr.to_le_bytes());
            }
            Instruction::AddI { dst, src, imm } => {
                bytecode.push(opcodes::ADDI);
                bytecode.push((dst << 4) | src);
                bytecode.extend_from_slice(&imm.to_le_bytes());
            }
        }

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_compile_simple() {
        let source = "LOADI R0, 10\nHALT";
        let program = Parser::parse(source).unwrap();
        let bytecode = Compiler::compile(&program).unwrap();

        // LOADI R0, 10 = [0x70, 0x00, 10 as le bytes (0x0A, 0x00, ...)]
        // HALT = [0x00]
        assert_eq!(bytecode[0], 0x70); // LOADI opcode
        assert_eq!(bytecode[1], 0x00); // R0
        assert_eq!(bytecode[2], 10); // immediate value (little-endian)
        assert_eq!(bytecode[10], 0x00); // HALT
        assert_eq!(bytecode.len(), 11);
    }

    #[test]
    fn test_compile_three_reg() {
        let source = "ADD R2, R0, R1";
        let program = Parser::parse(source).unwrap();
        let bytecode = Compiler::compile(&program).unwrap();

        // ADD R2, R0, R1 = [0x10, 0x20, 0x10]
        assert_eq!(bytecode, vec![0x10, 0x20, 0x10]);
    }

    #[test]
    fn test_compile_with_label() {
        let source = r#"
            .entry main
            main:
                LOADI R0, 10
                LOADI R5, loop_end
                JUMP R5
            loop_end:
                HALT
        "#;
        let program = Parser::parse(source).unwrap();
        let bytecode = Compiler::compile(&program).unwrap();

        // Should compile without error
        assert!(!bytecode.is_empty());

        // Check that LOADI R5, loop_end resolves to address 0x14 (20 decimal)
        // main: 0x00
        // LOADI R0, 10: 0x00-0x09 (10 bytes)
        // LOADI R5, loop_end: 0x0A-0x13 (10 bytes)
        // JUMP R5: 0x14-0x15 (2 bytes)
        // loop_end: 0x16
        // HALT: 0x16 (1 byte)

        // The second LOADI (at offset 10) should load address 0x16
        assert_eq!(bytecode[10], 0x70); // LOADI opcode
        assert_eq!(bytecode[11], 0x50); // R5 (5 << 4 = 0x50)
        assert_eq!(bytecode[12], 0x16); // Address of loop_end
    }

    #[test]
    fn test_undefined_label_error() {
        let source = "LOADI R5, undefined_label";
        let program = Parser::parse(source).unwrap();
        let result = Compiler::compile(&program);

        assert!(matches!(
            result,
            Err(CompileError::UndefinedLabel(ref s)) if s == "undefined_label"
        ));
    }

    #[test]
    fn test_duplicate_label_error() {
        let source = "main:\nHALT\nmain:\nHALT";
        let program = Parser::parse(source).unwrap();
        let result = Compiler::compile(&program);

        assert!(matches!(result, Err(CompileError::DuplicateLabel { .. })));
    }

    #[test]
    fn test_const_directive() {
        let source = r#"
            .const MAX_VALUE 100
            LOADI R0, MAX_VALUE
            HALT
        "#;
        let program = Parser::parse(source).unwrap();
        let bytecode = Compiler::compile(&program).unwrap();

        // LOADI should resolve MAX_VALUE to 100
        assert_eq!(bytecode[0], 0x70); // LOADI
        assert_eq!(bytecode[1], 0x00); // R0
        assert_eq!(bytecode[2], 100); // Constant value
    }

    #[test]
    fn test_forward_reference() {
        let source = r#"
            LOADI R5, end
            JUMP R5
            ADD R0, R1, R2
            end:
                HALT
        "#;
        let program = Parser::parse(source).unwrap();
        let bytecode = Compiler::compile(&program).unwrap();

        // Should compile successfully despite forward reference
        assert!(!bytecode.is_empty());
    }
}
