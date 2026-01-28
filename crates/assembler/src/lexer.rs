//! Tokenization for assembly language.
//!
//! Uses the logos crate for fast lexical analysis.

use logos::Logos;

/// Token types for assembly language
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]  // Skip whitespace
pub enum Token {
    // ========== Control Flow Instructions ==========
    #[token("HALT", ignore(ascii_case))]
    Halt,

    #[token("NOP", ignore(ascii_case))]
    Nop,

    #[token("JUMP", ignore(ascii_case))]
    Jump,

    #[token("JUMPI", ignore(ascii_case))]
    JumpI,

    #[token("CALL", ignore(ascii_case))]
    Call,

    #[token("RET", ignore(ascii_case))]
    Ret,

    #[token("REVERT", ignore(ascii_case))]
    Revert,

    // ========== Arithmetic Instructions ==========
    #[token("ADD", ignore(ascii_case))]
    Add,

    #[token("SUB", ignore(ascii_case))]
    Sub,

    #[token("MUL", ignore(ascii_case))]
    Mul,

    #[token("DIV", ignore(ascii_case))]
    Div,

    #[token("MOD", ignore(ascii_case))]
    Mod,

    #[token("ADDI", ignore(ascii_case))]
    AddI,

    // ========== Bitwise Instructions ==========
    #[token("AND", ignore(ascii_case))]
    And,

    #[token("OR", ignore(ascii_case))]
    Or,

    #[token("XOR", ignore(ascii_case))]
    Xor,

    #[token("NOT", ignore(ascii_case))]
    Not,

    #[token("SHL", ignore(ascii_case))]
    Shl,

    #[token("SHR", ignore(ascii_case))]
    Shr,

    // ========== Comparison Instructions ==========
    #[token("EQ", ignore(ascii_case))]
    Eq,

    #[token("NE", ignore(ascii_case))]
    Ne,

    #[token("LT", ignore(ascii_case))]
    Lt,

    #[token("GT", ignore(ascii_case))]
    Gt,

    #[token("LE", ignore(ascii_case))]
    Le,

    #[token("GE", ignore(ascii_case))]
    Ge,

    #[token("ISZERO", ignore(ascii_case))]
    IsZero,

    // ========== Memory Instructions ==========
    #[token("LOAD8", ignore(ascii_case))]
    Load8,

    #[token("LOAD64", ignore(ascii_case))]
    Load64,

    #[token("STORE8", ignore(ascii_case))]
    Store8,

    #[token("STORE64", ignore(ascii_case))]
    Store64,

    #[token("MSIZE", ignore(ascii_case))]
    MSize,

    #[token("MCOPY", ignore(ascii_case))]
    MCopy,

    // ========== Storage Instructions ==========
    #[token("SLOAD", ignore(ascii_case))]
    SLoad,

    #[token("SSTORE", ignore(ascii_case))]
    SStore,

    // ========== Immediate Instructions ==========
    #[token("LOADI", ignore(ascii_case))]
    LoadI,

    #[token("MOV", ignore(ascii_case))]
    Mov,

    // ========== Context Instructions ==========
    #[token("CALLER", ignore(ascii_case))]
    Caller,

    #[token("CALLVALUE", ignore(ascii_case))]
    CallValue,

    #[token("ADDRESS", ignore(ascii_case))]
    Address,

    #[token("BLOCKNUMBER", ignore(ascii_case))]
    BlockNumber,

    #[token("TIMESTAMP", ignore(ascii_case))]
    Timestamp,

    #[token("GAS", ignore(ascii_case))]
    Gas,

    // ========== Debug Instructions ==========
    #[token("LOG", ignore(ascii_case))]
    Log,

    // ========== Registers ==========
    #[regex(r"[Rr]([0-9]|1[0-5])", parse_register)]
    Register(u8),

    // ========== Numbers ==========
    #[regex(r"[0-9]+", parse_decimal)]
    Number(u64),

    #[regex(r"0x[0-9a-fA-F]+", parse_hex)]
    HexNumber(u64),

    // ========== Identifiers (labels, constants) ==========
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // ========== Directives ==========
    #[regex(r"\.[a-z]+", |lex| lex.slice()[1..].to_string())]
    Directive(String),

    // ========== Symbols ==========
    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    // ========== Comments (skipped) ==========
    #[regex(r";[^\n]*", logos::skip)]
    Comment,
}

/// Parse a register number from R0-R15
fn parse_register(lex: &mut logos::Lexer<Token>) -> Option<u8> {
    let slice = lex.slice();
    let num_str = &slice[1..]; // Skip 'R' or 'r'
    let num: u8 = num_str.parse().ok()?;
    if num <= 15 {
        Some(num)
    } else {
        None
    }
}

/// Parse a decimal number
fn parse_decimal(lex: &mut logos::Lexer<Token>) -> Option<u64> {
    lex.slice().parse().ok()
}

/// Parse a hexadecimal number (0x prefix)
fn parse_hex(lex: &mut logos::Lexer<Token>) -> Option<u64> {
    let slice = &lex.slice()[2..]; // Skip "0x"
    u64::from_str_radix(slice, 16).ok()
}

/// Lexer wrapper that tracks line numbers
pub struct Lexer<'source> {
    inner: logos::Lexer<'source, Token>,
    source: &'source str,
    last_pos: usize,
}

impl<'source> Lexer<'source> {
    /// Create a new lexer for the given source
    pub fn new(source: &'source str) -> Self {
        Self {
            inner: Token::lexer(source),
            source,
            last_pos: 0,
        }
    }

    /// Get the span of the current token
    pub fn span(&self) -> std::ops::Range<usize> {
        self.inner.span()
    }

    /// Get the slice of the current token
    pub fn slice(&self) -> &'source str {
        self.inner.slice()
    }

    /// Count line number based on position in source
    fn line_at_pos(&self, pos: usize) -> usize {
        1 + self.source[..pos].chars().filter(|c| *c == '\n').count()
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = (Token, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.inner.next()?;
        let span = self.inner.span();

        // Calculate line number based on position in source
        let line = self.line_at_pos(span.start);
        self.last_pos = span.end;

        // Logos returns Result<Token, ()> when error occurs
        let token = token.unwrap_or_else(|_| {
            // For errors, we'll create a special error identifier
            Token::Identifier("ERROR".to_string())
        });

        Some((token, line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let source = "LOADI R0, 10";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![
            Token::LoadI,
            Token::Register(0),
            Token::Comma,
            Token::Number(10),
        ]);
    }

    #[test]
    fn test_tokenize_with_label() {
        let source = "loop_start:\n    ADD R2, R0, R1";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![
            Token::Identifier("loop_start".to_string()),
            Token::Colon,
            Token::Add,
            Token::Register(2),
            Token::Comma,
            Token::Register(0),
            Token::Comma,
            Token::Register(1),
        ]);
    }

    #[test]
    fn test_tokenize_hex_number() {
        let source = "LOADI R0, 0xFF";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![
            Token::LoadI,
            Token::Register(0),
            Token::Comma,
            Token::HexNumber(255),
        ]);
    }

    #[test]
    fn test_skip_comments() {
        let source = "ADD R0, R1, R2  ; increment counter";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![
            Token::Add,
            Token::Register(0),
            Token::Comma,
            Token::Register(1),
            Token::Comma,
            Token::Register(2),
        ]);
    }

    #[test]
    fn test_directive() {
        let source = ".entry main";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![
            Token::Directive("entry".to_string()),
            Token::Identifier("main".to_string()),
        ]);
    }

    #[test]
    fn test_case_insensitive() {
        let source = "add ADD Add";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![Token::Add, Token::Add, Token::Add]);
    }

    #[test]
    fn test_register_range() {
        let source = "R0 R15";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t).collect();

        assert_eq!(tokens, vec![
            Token::Register(0),
            Token::Register(15),
        ]);

        // R16 is out of range and will be parsed as identifier
        let source2 = "R16";
        let tokens2: Vec<_> = Lexer::new(source2).map(|(t, _)| t).collect();
        assert!(matches!(tokens2[0], Token::Identifier(_)));
    }

    #[test]
    fn test_line_tracking() {
        let source = "LOADI R0, 10\nADD R1, R0, R0\nHALT";
        let items: Vec<_> = Lexer::new(source).collect();

        // First line tokens: LOADI R0, 10
        assert_eq!(items[0].1, 1); // LOADI on line 1
        assert_eq!(items[1].1, 1); // R0 on line 1
        assert_eq!(items[2].1, 1); // Comma on line 1
        assert_eq!(items[3].1, 1); // 10 on line 1

        // Second line tokens: ADD R1, R0, R0
        assert_eq!(items[4].1, 2); // ADD on line 2
        assert_eq!(items[5].1, 2); // R1 on line 2

        // Third line tokens: HALT
        assert_eq!(items[10].1, 3); // HALT on line 3
    }
}
