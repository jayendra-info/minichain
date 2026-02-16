//! Opcode definitions for the VM.

/// All VM opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Control Flow (0x00-0x0F)
    HALT = 0x00,
    NOP = 0x01,
    JUMP = 0x02,
    JUMPI = 0x03,
    CALL = 0x04,
    RET = 0x05,
    REVERT = 0x0F,

    // Arithmetic (0x10-0x1F)
    ADD = 0x10,
    SUB = 0x11,
    MUL = 0x12,
    DIV = 0x13,
    MOD = 0x14,
    ADDI = 0x15,

    // Bitwise (0x20-0x2F)
    AND = 0x20,
    OR = 0x21,
    XOR = 0x22,
    NOT = 0x23,
    SHL = 0x24,
    SHR = 0x25,

    // Comparison (0x30-0x3F)
    EQ = 0x30,
    NE = 0x31,
    LT = 0x32,
    GT = 0x33,
    LE = 0x34,
    GE = 0x35,
    ISZERO = 0x36,

    // Memory - RAM (0x40-0x4F)
    LOAD8 = 0x40,
    LOAD64 = 0x41,
    STORE8 = 0x42,
    STORE64 = 0x43,
    MSIZE = 0x44,
    MCOPY = 0x45,

    // Storage - Disk (0x50-0x5F)
    SLOAD = 0x50,
    SSTORE = 0x51,

    // Immediate (0x70-0x7F)
    LOADI = 0x70,
    MOV = 0x71,

    // Context (0x80-0x8F)
    CALLER = 0x80,
    CALLVALUE = 0x81,
    ADDRESS = 0x82,
    BLOCKNUMBER = 0x83,
    TIMESTAMP = 0x84,
    GAS = 0x85,

    // Debug (0xF0-0xFF)
    LOG = 0xF0,
}

impl Opcode {
    /// Parse a byte as an opcode.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Opcode::HALT),
            0x01 => Some(Opcode::NOP),
            0x02 => Some(Opcode::JUMP),
            0x03 => Some(Opcode::JUMPI),
            0x04 => Some(Opcode::CALL),
            0x05 => Some(Opcode::RET),
            0x0F => Some(Opcode::REVERT),

            0x10 => Some(Opcode::ADD),
            0x11 => Some(Opcode::SUB),
            0x12 => Some(Opcode::MUL),
            0x13 => Some(Opcode::DIV),
            0x14 => Some(Opcode::MOD),
            0x15 => Some(Opcode::ADDI),

            0x20 => Some(Opcode::AND),
            0x21 => Some(Opcode::OR),
            0x22 => Some(Opcode::XOR),
            0x23 => Some(Opcode::NOT),
            0x24 => Some(Opcode::SHL),
            0x25 => Some(Opcode::SHR),

            0x30 => Some(Opcode::EQ),
            0x31 => Some(Opcode::NE),
            0x32 => Some(Opcode::LT),
            0x33 => Some(Opcode::GT),
            0x34 => Some(Opcode::LE),
            0x35 => Some(Opcode::GE),
            0x36 => Some(Opcode::ISZERO),

            0x40 => Some(Opcode::LOAD8),
            0x41 => Some(Opcode::LOAD64),
            0x42 => Some(Opcode::STORE8),
            0x43 => Some(Opcode::STORE64),
            0x44 => Some(Opcode::MSIZE),
            0x45 => Some(Opcode::MCOPY),

            0x50 => Some(Opcode::SLOAD),
            0x51 => Some(Opcode::SSTORE),

            0x70 => Some(Opcode::LOADI),
            0x71 => Some(Opcode::MOV),

            0x80 => Some(Opcode::CALLER),
            0x81 => Some(Opcode::CALLVALUE),
            0x82 => Some(Opcode::ADDRESS),
            0x83 => Some(Opcode::BLOCKNUMBER),
            0x84 => Some(Opcode::TIMESTAMP),
            0x85 => Some(Opcode::GAS),

            0xF0 => Some(Opcode::LOG),

            _ => None,
        }
    }

    /// Get the number of bytes this instruction consumes (including opcode).
    pub fn instruction_size(&self) -> usize {
        match self {
            // No operands (1 byte total)
            Opcode::HALT | Opcode::NOP | Opcode::RET => 1,

            // One register (2 bytes: opcode + register)
            Opcode::JUMP
            | Opcode::NOT
            | Opcode::LOG
            | Opcode::MSIZE
            | Opcode::CALLER
            | Opcode::CALLVALUE
            | Opcode::ADDRESS
            | Opcode::BLOCKNUMBER
            | Opcode::TIMESTAMP
            | Opcode::GAS => 2,

            // Two registers (2 bytes: opcode + packed registers)
            Opcode::MOV
            | Opcode::LOAD8
            | Opcode::STORE8
            | Opcode::LOAD64
            | Opcode::STORE64
            | Opcode::SLOAD
            | Opcode::SSTORE
            | Opcode::ISZERO => 2,

            // Three registers (3 bytes: opcode + 2 packed register bytes)
            Opcode::ADD
            | Opcode::SUB
            | Opcode::MUL
            | Opcode::DIV
            | Opcode::MOD
            | Opcode::AND
            | Opcode::OR
            | Opcode::XOR
            | Opcode::SHL
            | Opcode::SHR
            | Opcode::EQ
            | Opcode::NE
            | Opcode::LT
            | Opcode::GT
            | Opcode::LE
            | Opcode::GE
            | Opcode::MCOPY
            | Opcode::JUMPI => 3,

            // Register + 64-bit immediate (10 bytes)
            Opcode::LOADI => 10, // 1 (opcode) + 1 (reg) + 8 (immediate)

            // Fallback
            _ => 2,
        }
    }
}
