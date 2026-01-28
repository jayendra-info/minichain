//! Assembly to bytecode compiler for minichain.
//!
//! This crate provides a complete assembler that compiles human-readable
//! assembly language into compact bytecode for the minichain VM.
//!
//! # Example
//!
//! ```
//! use minichain_assembler::assemble;
//!
//! let source = r#"
//!     .entry main
//!
//!     main:
//!         LOADI R0, 10
//!         LOADI R1, 20
//!         ADD R2, R0, R1
//!         LOG R2
//!         HALT
//! "#;
//!
//! let bytecode = assemble(source).expect("failed to assemble");
//! println!("Compiled {} bytes of bytecode", bytecode.len());
//! ```
//!
//! # Pipeline
//!
//! The assembler uses a three-stage pipeline:
//!
//! 1. **Lexer** - Tokenizes the source code into a stream of tokens
//! 2. **Parser** - Builds an Abstract Syntax Tree (AST) from the tokens
//! 3. **Compiler** - Performs two-pass compilation to emit bytecode:
//!    - Pass 1: Collect label addresses and constants
//!    - Pass 2: Emit bytecode with resolved label references
//!
//! # Assembly Language Features
//!
//! - **60+ instructions** - Full VM instruction set support
//! - **Labels** - Symbolic jump targets
//! - **Directives** - `.entry` for entry points, `.const` for constants
//! - **Comments** - Semicolon-style comments
//! - **Case-insensitive** - Instructions can be uppercase or lowercase
//! - **Hex literals** - Support for `0x` prefixed hexadecimal numbers

pub mod compiler;
pub mod lexer;
pub mod parser;

use thiserror::Error;

/// Assembler errors
#[derive(Error, Debug)]
pub enum AssemblerError {
    #[error("parse error: {0}")]
    Parse(#[from] parser::ParseError),

    #[error("compile error: {0}")]
    Compile(#[from] compiler::CompileError),
}

pub type Result<T> = std::result::Result<T, AssemblerError>;

/// Assemble source code into bytecode
///
/// This is the main entry point for the assembler. It performs lexing,
/// parsing, and compilation in a single function call.
///
/// # Example
///
/// ```
/// use minichain_assembler::assemble;
///
/// let bytecode = assemble("LOADI R0, 42\nHALT").unwrap();
/// assert_eq!(bytecode[0], 0x70); // LOADI opcode
/// assert_eq!(bytecode[bytecode.len() - 1], 0x00); // HALT opcode
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The source contains invalid syntax
/// - Labels are undefined or duplicated
/// - Registers are out of range
pub fn assemble(source: &str) -> Result<Vec<u8>> {
    // Parse the source into an AST
    let program = parser::Parser::parse(source)?;

    // Compile the AST to bytecode
    let bytecode = compiler::Compiler::compile(&program)?;

    Ok(bytecode)
}

/// Assemble source code and return both the AST and bytecode
///
/// This function is useful for debugging or when you need both the
/// parsed program structure and the compiled bytecode.
///
/// # Example
///
/// ```
/// use minichain_assembler::assemble_with_ast;
///
/// let (program, bytecode) = assemble_with_ast("LOADI R0, 10\nHALT").unwrap();
/// println!("Entry point: {:?}", program.entry_point);
/// println!("Bytecode size: {} bytes", bytecode.len());
/// ```
pub fn assemble_with_ast(source: &str) -> Result<(parser::Program, Vec<u8>)> {
    let program = parser::Parser::parse(source)?;
    let bytecode = compiler::Compiler::compile(&program)?;
    Ok((program, bytecode))
}

// Re-export commonly used types
pub use compiler::{CompileError, Compiler};
pub use lexer::{Lexer, Token};
pub use parser::{Directive, Instruction, ParseError, Parser, Program, Statement};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assemble_simple() {
        let source = "LOADI R0, 10\nHALT";
        let bytecode = assemble(source).unwrap();
        assert!(bytecode.len() > 0);
    }

    #[test]
    fn test_assemble_with_labels() {
        let source = r#"
            .entry main
            main:
                LOADI R0, 10
                LOADI R5, end
                JUMP R5
            end:
                HALT
        "#;
        let bytecode = assemble(source).unwrap();
        assert!(bytecode.len() > 0);
    }

    #[test]
    fn test_assemble_error_undefined_label() {
        let source = "LOADI R5, undefined";
        let result = assemble(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_assemble_with_ast() {
        let source = ".entry main\nmain:\nHALT";
        let (program, bytecode) = assemble_with_ast(source).unwrap();

        assert_eq!(program.entry_point, Some("main".to_string()));
        assert!(bytecode.len() > 0);
    }

    #[test]
    fn test_complete_program() {
        let source = r#"
            ; Counter contract - increment storage value
            .entry main

            main:
                LOADI R0, 0          ; storage slot 0
                SLOAD R1, R0         ; load current value
                LOADI R2, 1          ; increment by 1
                ADD R1, R1, R2       ; increment
                SSTORE R0, R1        ; store back
                HALT                 ; done
        "#;

        let bytecode = assemble(source).unwrap();
        // Verify it compiles without error
        assert!(bytecode.len() > 0);

        // Check key opcodes are present
        assert_eq!(bytecode[0], 0x70); // First LOADI
        assert_eq!(bytecode[10], 0x50); // SLOAD
        assert!(bytecode.contains(&0x00)); // HALT at end
    }
}
