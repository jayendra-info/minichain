//! Parse assembly into Abstract Syntax Tree (AST).

use crate::lexer::{Lexer, Token};
use thiserror::Error;

/// Parse errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("unexpected token at line {line}: expected {expected}, found {found}")]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
    },

    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("invalid register at line {line}: {register}")]
    InvalidRegister { register: String, line: usize },

    #[error("duplicate label '{label}' at line {line} (first defined at line {first_line})")]
    DuplicateLabel {
        label: String,
        line: usize,
        first_line: usize,
    },
}

pub type Result<T> = std::result::Result<T, ParseError>;

/// Top-level program structure
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub entry_point: Option<String>,
}

/// Statement types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Label(String),
    Instruction(Instruction),
    Directive(Directive),
}

/// Directive types
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    Entry(String),
    Const(String, u64),
}

/// Instruction types
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Control flow
    Halt,
    Nop,
    Jump { target: u8 },
    JumpI { cond: u8, target: u8 },
    Call { target: u8 },
    Ret,
    Revert,

    // Arithmetic
    Add { dst: u8, s1: u8, s2: u8 },
    Sub { dst: u8, s1: u8, s2: u8 },
    Mul { dst: u8, s1: u8, s2: u8 },
    Div { dst: u8, s1: u8, s2: u8 },
    Mod { dst: u8, s1: u8, s2: u8 },
    AddI { dst: u8, src: u8, imm: u64 },

    // Bitwise
    And { dst: u8, s1: u8, s2: u8 },
    Or { dst: u8, s1: u8, s2: u8 },
    Xor { dst: u8, s1: u8, s2: u8 },
    Not { dst: u8, src: u8 },
    Shl { dst: u8, s1: u8, s2: u8 },
    Shr { dst: u8, s1: u8, s2: u8 },

    // Comparison
    Eq { dst: u8, s1: u8, s2: u8 },
    Ne { dst: u8, s1: u8, s2: u8 },
    Lt { dst: u8, s1: u8, s2: u8 },
    Gt { dst: u8, s1: u8, s2: u8 },
    Le { dst: u8, s1: u8, s2: u8 },
    Ge { dst: u8, s1: u8, s2: u8 },
    IsZero { dst: u8, src: u8 },

    // Memory
    Load8 { dst: u8, addr: u8 },
    Load64 { dst: u8, addr: u8 },
    Store8 { addr: u8, src: u8 },
    Store64 { addr: u8, src: u8 },
    MSize { dst: u8 },
    MCopy { dst: u8, src: u8, len: u8 },

    // Storage
    SLoad { dst: u8, key: u8 },
    SStore { key: u8, value: u8 },

    // Immediate
    LoadI { dst: u8, value: u64 },
    LoadILabel { dst: u8, label: String },
    Mov { dst: u8, src: u8 },

    // Context
    Caller { dst: u8 },
    CallValue { dst: u8 },
    Address { dst: u8 },
    BlockNumber { dst: u8 },
    Timestamp { dst: u8 },
    Gas { dst: u8 },

    // Debug
    Log { src: u8 },
}

impl Instruction {
    /// Get the size of this instruction in bytes
    pub fn byte_size(&self) -> usize {
        match self {
            // No operands (1 byte: opcode)
            Instruction::Halt | Instruction::Nop | Instruction::Ret | Instruction::Revert => 1,

            // Single register (2 bytes: opcode + register)
            Instruction::Jump { .. }
            | Instruction::Call { .. }
            | Instruction::Not { .. }
            | Instruction::MSize { .. }
            | Instruction::Caller { .. }
            | Instruction::CallValue { .. }
            | Instruction::Address { .. }
            | Instruction::BlockNumber { .. }
            | Instruction::Timestamp { .. }
            | Instruction::Gas { .. }
            | Instruction::Log { .. } => 2,

            // Two registers (2 bytes: opcode + packed registers)
            Instruction::JumpI { .. }
            | Instruction::Mov { .. }
            | Instruction::Load8 { .. }
            | Instruction::Load64 { .. }
            | Instruction::Store8 { .. }
            | Instruction::Store64 { .. }
            | Instruction::SLoad { .. }
            | Instruction::SStore { .. }
            | Instruction::IsZero { .. } => 2,

            // Three registers (3 bytes: opcode + 2 bytes packed registers)
            Instruction::Add { .. }
            | Instruction::Sub { .. }
            | Instruction::Mul { .. }
            | Instruction::Div { .. }
            | Instruction::Mod { .. }
            | Instruction::And { .. }
            | Instruction::Or { .. }
            | Instruction::Xor { .. }
            | Instruction::Shl { .. }
            | Instruction::Shr { .. }
            | Instruction::Eq { .. }
            | Instruction::Ne { .. }
            | Instruction::Lt { .. }
            | Instruction::Gt { .. }
            | Instruction::Le { .. }
            | Instruction::Ge { .. }
            | Instruction::MCopy { .. } => 3,

            // Register + immediate (10 bytes: opcode + register + u64)
            Instruction::LoadI { .. }
            | Instruction::LoadILabel { .. }
            | Instruction::AddI { .. } => 10,
        }
    }
}

/// Parser state
pub struct Parser<'source> {
    tokens: Vec<(Token, usize)>,
    position: usize,
    source: &'source str,
}

impl<'source> Parser<'source> {
    /// Create a new parser
    pub fn new(source: &'source str) -> Self {
        let tokens: Vec<_> = Lexer::new(source).collect();
        Self {
            tokens,
            position: 0,
            source,
        }
    }

    /// Parse the entire program
    pub fn parse(source: &'source str) -> Result<Program> {
        let mut parser = Self::new(source);
        parser.parse_program()
    }

    /// Parse a program
    fn parse_program(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        let mut entry_point = None;

        while !self.is_at_end() {
            let stmt = self.parse_statement()?;

            // Track .entry directive
            if let Statement::Directive(Directive::Entry(ref name)) = stmt {
                entry_point = Some(name.clone());
            }

            statements.push(stmt);
        }

        Ok(Program {
            statements,
            entry_point,
        })
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> Result<Statement> {
        let (token, line) = self.peek();

        match token {
            Token::Directive(name) => {
                self.advance();
                self.parse_directive(&name, line)
            }
            Token::Identifier(name) => {
                // Check if it's a label (followed by colon)
                if self.peek_ahead(1).map(|(t, _)| t) == Some(&Token::Colon) {
                    self.advance(); // consume identifier
                    self.advance(); // consume colon
                    Ok(Statement::Label(name.clone()))
                } else {
                    // Check for error token (from lexer errors)
                    if name == "ERROR" {
                        return Err(ParseError::UnexpectedToken {
                            expected: "valid token".to_string(),
                            found: "invalid character".to_string(),
                            line,
                        });
                    }
                    Err(ParseError::UnexpectedToken {
                        expected: "instruction or label".to_string(),
                        found: format!("identifier '{}'", name),
                        line,
                    })
                }
            }
            _ => {
                let inst = self.parse_instruction()?;
                Ok(Statement::Instruction(inst))
            }
        }
    }

    /// Parse a directive
    fn parse_directive(&mut self, name: &str, line: usize) -> Result<Statement> {
        match name {
            "entry" => {
                let label = self.expect_identifier()?;
                Ok(Statement::Directive(Directive::Entry(label)))
            }
            "const" => {
                let name = self.expect_identifier()?;
                let value = self.expect_number()?;
                Ok(Statement::Directive(Directive::Const(name, value)))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "entry or const".to_string(),
                found: format!("directive '{}'", name),
                line,
            }),
        }
    }

    /// Parse an instruction
    fn parse_instruction(&mut self) -> Result<Instruction> {
        let (token, line) = self.advance();

        match token {
            // No operands
            Token::Halt => Ok(Instruction::Halt),
            Token::Nop => Ok(Instruction::Nop),
            Token::Ret => Ok(Instruction::Ret),
            Token::Revert => Ok(Instruction::Revert),

            // Single register
            Token::Jump => {
                let target = self.expect_register()?;
                Ok(Instruction::Jump { target })
            }
            Token::Call => {
                let target = self.expect_register()?;
                Ok(Instruction::Call { target })
            }
            Token::Log => {
                let src = self.expect_register()?;
                Ok(Instruction::Log { src })
            }

            // Two registers
            Token::JumpI => {
                let cond = self.expect_register()?;
                self.expect_comma()?;
                let target = self.expect_register()?;
                Ok(Instruction::JumpI { cond, target })
            }
            Token::Mov => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let src = self.expect_register()?;
                Ok(Instruction::Mov { dst, src })
            }
            Token::Not => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let src = self.expect_register()?;
                Ok(Instruction::Not { dst, src })
            }
            Token::IsZero => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let src = self.expect_register()?;
                Ok(Instruction::IsZero { dst, src })
            }
            Token::Load8 => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let addr = self.expect_register()?;
                Ok(Instruction::Load8 { dst, addr })
            }
            Token::Load64 => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let addr = self.expect_register()?;
                Ok(Instruction::Load64 { dst, addr })
            }
            Token::Store8 => {
                let addr = self.expect_register()?;
                self.expect_comma()?;
                let src = self.expect_register()?;
                Ok(Instruction::Store8 { addr, src })
            }
            Token::Store64 => {
                let addr = self.expect_register()?;
                self.expect_comma()?;
                let src = self.expect_register()?;
                Ok(Instruction::Store64 { addr, src })
            }
            Token::SLoad => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let key = self.expect_register()?;
                Ok(Instruction::SLoad { dst, key })
            }
            Token::SStore => {
                let key = self.expect_register()?;
                self.expect_comma()?;
                let value = self.expect_register()?;
                Ok(Instruction::SStore { key, value })
            }
            Token::MSize => {
                let dst = self.expect_register()?;
                Ok(Instruction::MSize { dst })
            }
            Token::Caller => {
                let dst = self.expect_register()?;
                Ok(Instruction::Caller { dst })
            }
            Token::CallValue => {
                let dst = self.expect_register()?;
                Ok(Instruction::CallValue { dst })
            }
            Token::Address => {
                let dst = self.expect_register()?;
                Ok(Instruction::Address { dst })
            }
            Token::BlockNumber => {
                let dst = self.expect_register()?;
                Ok(Instruction::BlockNumber { dst })
            }
            Token::Timestamp => {
                let dst = self.expect_register()?;
                Ok(Instruction::Timestamp { dst })
            }
            Token::Gas => {
                let dst = self.expect_register()?;
                Ok(Instruction::Gas { dst })
            }

            // Three registers
            Token::Add => self.parse_three_reg(|d, s1, s2| Instruction::Add { dst: d, s1, s2 }),
            Token::Sub => self.parse_three_reg(|d, s1, s2| Instruction::Sub { dst: d, s1, s2 }),
            Token::Mul => self.parse_three_reg(|d, s1, s2| Instruction::Mul { dst: d, s1, s2 }),
            Token::Div => self.parse_three_reg(|d, s1, s2| Instruction::Div { dst: d, s1, s2 }),
            Token::Mod => self.parse_three_reg(|d, s1, s2| Instruction::Mod { dst: d, s1, s2 }),
            Token::And => self.parse_three_reg(|d, s1, s2| Instruction::And { dst: d, s1, s2 }),
            Token::Or => self.parse_three_reg(|d, s1, s2| Instruction::Or { dst: d, s1, s2 }),
            Token::Xor => self.parse_three_reg(|d, s1, s2| Instruction::Xor { dst: d, s1, s2 }),
            Token::Shl => self.parse_three_reg(|d, s1, s2| Instruction::Shl { dst: d, s1, s2 }),
            Token::Shr => self.parse_three_reg(|d, s1, s2| Instruction::Shr { dst: d, s1, s2 }),
            Token::Eq => self.parse_three_reg(|d, s1, s2| Instruction::Eq { dst: d, s1, s2 }),
            Token::Ne => self.parse_three_reg(|d, s1, s2| Instruction::Ne { dst: d, s1, s2 }),
            Token::Lt => self.parse_three_reg(|d, s1, s2| Instruction::Lt { dst: d, s1, s2 }),
            Token::Gt => self.parse_three_reg(|d, s1, s2| Instruction::Gt { dst: d, s1, s2 }),
            Token::Le => self.parse_three_reg(|d, s1, s2| Instruction::Le { dst: d, s1, s2 }),
            Token::Ge => self.parse_three_reg(|d, s1, s2| Instruction::Ge { dst: d, s1, s2 }),
            Token::MCopy => {
                self.parse_three_reg(|d, s, l| Instruction::MCopy { dst: d, src: s, len: l })
            }

            // Register + immediate
            Token::LoadI => {
                let dst = self.expect_register()?;
                self.expect_comma()?;

                // Check if next token is a number or identifier (label)
                let (token, _line) = self.peek();
                match token {
                    Token::Number(n) | Token::HexNumber(n) => {
                        let value = n;
                        self.advance();
                        Ok(Instruction::LoadI { dst, value })
                    }
                    Token::Identifier(label) => {
                        let label = label.clone();
                        self.advance();
                        Ok(Instruction::LoadILabel { dst, label })
                    }
                    _ => {
                        let (_, line) = self.peek();
                        Err(ParseError::UnexpectedToken {
                            expected: "number or label".to_string(),
                            found: format!("{:?}", token),
                            line,
                        })
                    }
                }
            }
            Token::AddI => {
                let dst = self.expect_register()?;
                self.expect_comma()?;
                let src = self.expect_register()?;
                self.expect_comma()?;
                let imm = self.expect_number()?;
                Ok(Instruction::AddI { dst, src, imm })
            }

            _ => Err(ParseError::UnexpectedToken {
                expected: "instruction".to_string(),
                found: format!("{:?}", token),
                line,
            }),
        }
    }

    /// Helper to parse three-register instruction
    fn parse_three_reg<F>(&mut self, f: F) -> Result<Instruction>
    where
        F: FnOnce(u8, u8, u8) -> Instruction,
    {
        let dst = self.expect_register()?;
        self.expect_comma()?;
        let s1 = self.expect_register()?;
        self.expect_comma()?;
        let s2 = self.expect_register()?;
        Ok(f(dst, s1, s2))
    }

    /// Expect a register
    fn expect_register(&mut self) -> Result<u8> {
        let (token, line) = self.advance();
        match token {
            Token::Register(r) => Ok(r),
            _ => Err(ParseError::UnexpectedToken {
                expected: "register (R0-R15)".to_string(),
                found: format!("{:?}", token),
                line,
            }),
        }
    }

    /// Expect a number
    fn expect_number(&mut self) -> Result<u64> {
        let (token, line) = self.advance();
        match token {
            Token::Number(n) | Token::HexNumber(n) => Ok(n),
            _ => Err(ParseError::UnexpectedToken {
                expected: "number".to_string(),
                found: format!("{:?}", token),
                line,
            }),
        }
    }

    /// Expect an identifier
    fn expect_identifier(&mut self) -> Result<String> {
        let (token, line) = self.advance();
        match token {
            Token::Identifier(s) => Ok(s.clone()),
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: format!("{:?}", token),
                line,
            }),
        }
    }

    /// Expect a comma
    fn expect_comma(&mut self) -> Result<()> {
        let (token, line) = self.advance();
        match token {
            Token::Comma => Ok(()),
            _ => Err(ParseError::UnexpectedToken {
                expected: "comma".to_string(),
                found: format!("{:?}", token),
                line,
            }),
        }
    }

    /// Advance to next token
    fn advance(&mut self) -> (Token, usize) {
        if self.is_at_end() {
            (Token::Halt, 0) // Return a dummy token
        } else {
            let token = self.tokens[self.position].clone();
            self.position += 1;
            token
        }
    }

    /// Peek at current token
    fn peek(&self) -> (Token, usize) {
        if self.is_at_end() {
            (Token::Halt, 0) // Return a dummy token
        } else {
            self.tokens[self.position].clone()
        }
    }

    /// Peek ahead N tokens
    fn peek_ahead(&self, n: usize) -> Option<&(Token, usize)> {
        self.tokens.get(self.position + n)
    }

    /// Check if at end of input
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let source = "LOADI R0, 10\nHALT";
        let program = Parser::parse(source).unwrap();

        assert_eq!(program.statements.len(), 2);
        assert!(matches!(
            program.statements[0],
            Statement::Instruction(Instruction::LoadI { dst: 0, value: 10 })
        ));
        assert!(matches!(
            program.statements[1],
            Statement::Instruction(Instruction::Halt)
        ));
    }

    #[test]
    fn test_parse_with_label() {
        let source = "main:\n    LOADI R0, 10\n    HALT";
        let program = Parser::parse(source).unwrap();

        assert_eq!(program.statements.len(), 3);
        assert!(matches!(
            program.statements[0],
            Statement::Label(ref s) if s == "main"
        ));
    }

    #[test]
    fn test_parse_three_reg() {
        let source = "ADD R2, R0, R1";
        let program = Parser::parse(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(
            program.statements[0],
            Statement::Instruction(Instruction::Add {
                dst: 2,
                s1: 0,
                s2: 1
            })
        ));
    }

    #[test]
    fn test_parse_entry_directive() {
        let source = ".entry main\nmain:\n    HALT";
        let program = Parser::parse(source).unwrap();

        assert_eq!(program.entry_point, Some("main".to_string()));
        assert_eq!(program.statements.len(), 3);
    }

    #[test]
    fn test_parse_loadi_label() {
        let source = "LOADI R5, loop_start";
        let program = Parser::parse(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(
            program.statements[0],
            Statement::Instruction(Instruction::LoadILabel { dst: 5, ref label })
            if label == "loop_start"
        ));
    }

    #[test]
    fn test_instruction_byte_size() {
        assert_eq!(Instruction::Halt.byte_size(), 1);
        assert_eq!(Instruction::Jump { target: 0 }.byte_size(), 2);
        assert_eq!(
            Instruction::Add {
                dst: 0,
                s1: 1,
                s2: 2
            }
            .byte_size(),
            3
        );
        assert_eq!(Instruction::LoadI { dst: 0, value: 10 }.byte_size(), 10);
    }
}
