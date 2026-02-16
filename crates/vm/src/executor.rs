//! VM execution loop.

use crate::{
    gas::{GasCosts, GasMeter},
    memory::{Memory, Registers},
    opcodes::Opcode,
};
use minichain_core::Address;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    #[error("Out of gas: required {required}, remaining {remaining}")]
    OutOfGas { required: u64, remaining: u64 },

    #[error("Invalid opcode: 0x{0:02X}")]
    InvalidOpcode(u8),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Memory overflow")]
    MemoryOverflow,

    #[error("Invalid jump destination: {0}")]
    InvalidJump(usize),

    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Execution reverted")]
    Reverted,
}

/// Execution result.
pub struct ExecutionResult {
    pub success: bool,
    pub gas_used: u64,
    pub return_data: Vec<u8>,
    pub logs: Vec<u64>, // LOG opcode outputs
}

/// The virtual machine state.
pub struct Vm {
    registers: Registers,
    memory: Memory,
    pc: usize,
    gas: GasMeter,
    bytecode: Vec<u8>,
    halted: bool,

    // Context
    caller: Address,
    address: Address,
    call_value: u64,
    block_number: u64,
    timestamp: u64,

    // Storage backend
    storage: Option<Box<dyn StorageBackend>>,

    // Outputs
    logs: Vec<u64>,
}

impl Vm {
    /// Create a new VM with the given bytecode and gas limit.
    pub fn new(
        bytecode: Vec<u8>,
        gas_limit: u64,
        caller: Address,
        address: Address,
        call_value: u64,
    ) -> Self {
        Self {
            registers: Registers::new(),
            memory: Memory::new(1024 * 1024), // 1MB max
            pc: 0,
            gas: GasMeter::new(gas_limit),
            bytecode,
            halted: false,
            caller,
            address,
            call_value,
            block_number: 0,
            timestamp: 0,
            storage: None,
            logs: Vec::new(),
        }
    }

    /// Create a new VM with additional context fields.
    pub fn new_with_context(
        bytecode: Vec<u8>,
        gas_limit: u64,
        caller: Address,
        address: Address,
        call_value: u64,
        block_number: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            registers: Registers::new(),
            memory: Memory::new(1024 * 1024),
            pc: 0,
            gas: GasMeter::new(gas_limit),
            bytecode,
            halted: false,
            caller,
            address,
            call_value,
            block_number,
            timestamp,
            storage: None,
            logs: Vec::new(),
        }
    }

    /// Set the storage backend.
    pub fn set_storage(&mut self, storage: Box<dyn StorageBackend>) {
        self.storage = Some(storage);
    }

    /// Set the block context.
    pub fn set_block_context(&mut self, block_number: u64, timestamp: u64) {
        self.block_number = block_number;
        self.timestamp = timestamp;
    }

    /// Get current register values for tracing.
    pub fn get_registers(&self) -> &[u64; 16] {
        self.registers.values()
    }

    /// Get remaining gas.
    pub fn gas_remaining(&self) -> u64 {
        self.gas.remaining()
    }

    /// Run the VM until it halts or runs out of gas.
    pub fn run(&mut self) -> Result<ExecutionResult, VmError> {
        while !self.halted && self.pc < self.bytecode.len() {
            self.step()?;
        }

        Ok(ExecutionResult {
            success: self.halted, // HALT = success, out of bounds = failure
            gas_used: self.gas.used(),
            return_data: Vec::new(), // TODO: implement return data
            logs: std::mem::take(&mut self.logs),
        })
    }

    /// Execute a single instruction.
    fn step(&mut self) -> Result<(), VmError> {
        // Fetch
        let opcode_byte = self.bytecode[self.pc];
        let opcode = Opcode::from_byte(opcode_byte).ok_or(VmError::InvalidOpcode(opcode_byte))?;

        // Decode & Execute
        match opcode {
            Opcode::HALT => {
                self.gas.consume(GasCosts::ZERO)?;
                self.halted = true;
            }

            Opcode::NOP => {
                self.gas.consume(GasCosts::ZERO)?;
                self.pc += 1;
            }

            Opcode::ADD => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                let result = self.registers.get(s1).wrapping_add(self.registers.get(s2));
                self.registers.set(dst, result);
                self.pc += 3;
            }

            Opcode::SUB => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                let result = self.registers.get(s1).wrapping_sub(self.registers.get(s2));
                self.registers.set(dst, result);
                self.pc += 3;
            }

            Opcode::MUL => {
                self.gas.consume(GasCosts::LOW)?;
                let (dst, s1, s2) = self.decode_rrr();
                let result = self.registers.get(s1).wrapping_mul(self.registers.get(s2));
                self.registers.set(dst, result);
                self.pc += 3;
            }

            Opcode::DIV => {
                self.gas.consume(GasCosts::MID)?;
                let (dst, s1, s2) = self.decode_rrr();
                let divisor = self.registers.get(s2);
                if divisor == 0 {
                    return Err(VmError::DivisionByZero);
                }
                self.registers.set(dst, self.registers.get(s1) / divisor);
                self.pc += 3;
            }

            Opcode::MOD => {
                self.gas.consume(GasCosts::MID)?;
                let (dst, s1, s2) = self.decode_rrr();
                let divisor = self.registers.get(s2);
                if divisor == 0 {
                    return Err(VmError::DivisionByZero);
                }
                self.registers.set(dst, self.registers.get(s1) % divisor);
                self.pc += 3;
            }

            Opcode::ADDI => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, src) = self.decode_rr();
                let immediate = self.decode_imm32() as u64;
                let result = self.registers.get(src).wrapping_add(immediate);
                self.registers.set(dst, result);
                self.pc += 6;
            }

            Opcode::AND => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers
                    .set(dst, self.registers.get(s1) & self.registers.get(s2));
                self.pc += 3;
            }

            Opcode::OR => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers
                    .set(dst, self.registers.get(s1) | self.registers.get(s2));
                self.pc += 3;
            }

            Opcode::XOR => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers
                    .set(dst, self.registers.get(s1) ^ self.registers.get(s2));
                self.pc += 3;
            }

            Opcode::NOT => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers.set(dst, !self.registers.get(dst));
                self.pc += 2;
            }

            Opcode::SHL => {
                self.gas.consume(GasCosts::MID)?;
                let (dst, s1, s2) = self.decode_rrr();
                let shift = self.registers.get(s2) & 0x3F;
                self.registers.set(dst, self.registers.get(s1) << shift);
                self.pc += 3;
            }

            Opcode::SHR => {
                self.gas.consume(GasCosts::MID)?;
                let (dst, s1, s2) = self.decode_rrr();
                let shift = self.registers.get(s2) & 0x3F;
                self.registers.set(dst, self.registers.get(s1) >> shift);
                self.pc += 3;
            }

            Opcode::EQ => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers.set(
                    dst,
                    (self.registers.get(s1) == self.registers.get(s2)) as u64,
                );
                self.pc += 3;
            }

            Opcode::NE => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers.set(
                    dst,
                    (self.registers.get(s1) != self.registers.get(s2)) as u64,
                );
                self.pc += 3;
            }

            Opcode::LT => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers.set(
                    dst,
                    (self.registers.get(s1) < self.registers.get(s2)) as u64,
                );
                self.pc += 3;
            }

            Opcode::GT => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers.set(
                    dst,
                    (self.registers.get(s1) > self.registers.get(s2)) as u64,
                );
                self.pc += 3;
            }

            Opcode::LE => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers.set(
                    dst,
                    (self.registers.get(s1) <= self.registers.get(s2)) as u64,
                );
                self.pc += 3;
            }

            Opcode::GE => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, s1, s2) = self.decode_rrr();
                self.registers.set(
                    dst,
                    (self.registers.get(s1) >= self.registers.get(s2)) as u64,
                );
                self.pc += 3;
            }

            Opcode::ISZERO => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers
                    .set(dst, (self.registers.get(dst) == 0) as u64);
                self.pc += 2;
            }

            Opcode::LOADI => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                let immediate = self.decode_imm64();
                self.registers.set(dst, immediate);
                self.pc += 10;
            }

            Opcode::MOV => {
                self.gas.consume(GasCosts::BASE)?;
                let (dst, src) = self.decode_rr();
                self.registers.set(dst, self.registers.get(src));
                self.pc += 2;
            }

            Opcode::JUMP => {
                self.gas.consume(GasCosts::JUMP)?;
                let target = self.decode_r();
                let addr = self.registers.get(target) as usize;
                if addr >= self.bytecode.len() {
                    return Err(VmError::InvalidJump(addr));
                }
                self.pc = addr;
            }

            Opcode::JUMPI => {
                self.gas.consume(GasCosts::JUMP)?;
                let (cond, target) = self.decode_rr();
                if self.registers.get(cond) != 0 {
                    let addr = self.registers.get(target) as usize;
                    if addr >= self.bytecode.len() {
                        return Err(VmError::InvalidJump(addr));
                    }
                    self.pc = addr;
                } else {
                    self.pc += 2;
                }
            }

            Opcode::RET => {
                self.gas.consume(GasCosts::ZERO)?;
                self.halted = true;
                self.pc += 1;
            }

            Opcode::REVERT => {
                self.gas.consume(GasCosts::ZERO)?;
                self.halted = true;
                self.pc += 1;
                return Err(VmError::Reverted);
            }

            Opcode::CALL => {
                self.gas.consume(GasCosts::CALL)?;
                self.pc += 1;
            }

            Opcode::LOG => {
                self.gas.consume(GasCosts::BASE)?;
                let src = self.decode_r();
                self.logs.push(self.registers.get(src));
                self.pc += 2;
            }

            Opcode::LOAD8 => {
                self.gas.consume(GasCosts::MEMORY_READ)?;
                let (dst, addr_reg) = self.decode_rr();
                let addr = self.registers.get(addr_reg) as u32;
                let value = self.memory.load8(addr) as u64;
                self.registers.set(dst, value);
                self.pc += 2;
            }

            Opcode::LOAD64 => {
                self.gas.consume(GasCosts::MEMORY_READ)?;
                let (dst, addr_reg) = self.decode_rr();
                let addr = self.registers.get(addr_reg) as u32;
                let value = self.memory.load64(addr);
                self.registers.set(dst, value);
                self.pc += 2;
            }

            Opcode::STORE8 => {
                self.gas.consume(GasCosts::MEMORY_WRITE)?;
                let (addr_reg, value_reg) = self.decode_rr();
                let addr = self.registers.get(addr_reg) as u32;
                let value = self.registers.get(value_reg) as u8;
                self.memory.store8(addr, value)?;
                self.pc += 2;
            }

            Opcode::STORE64 => {
                self.gas.consume(GasCosts::MEMORY_WRITE)?;
                let (addr_reg, value_reg) = self.decode_rr();
                let addr = self.registers.get(addr_reg) as u32;
                let value = self.registers.get(value_reg);
                self.memory.store64(addr, value)?;
                self.pc += 2;
            }

            Opcode::MSIZE => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers.set(dst, self.memory.size() as u64);
                self.pc += 2;
            }

            Opcode::MCOPY => {
                self.gas.consume(GasCosts::MEMORY_READ)?;
                let (dst_reg, src_reg, len_reg) = self.decode_rrr();
                let dest = self.registers.get(dst_reg) as u32;
                let src = self.registers.get(src_reg) as u32;
                let length = self.registers.get(len_reg) as u32;
                self.memory.mcopy(dest, src, length)?;
                self.pc += 3;
            }

            Opcode::SLOAD => {
                self.execute_sload()?;
            }

            Opcode::SSTORE => {
                self.execute_sstore_internal()?;
            }

            Opcode::CALLER => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                let caller_bytes = self.caller.as_bytes();
                let mut value = [0u8; 8];
                value.copy_from_slice(&caller_bytes[..8]);
                self.registers.set(dst, u64::from_le_bytes(value));
                self.pc += 2;
            }

            Opcode::CALLVALUE => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers.set(dst, self.call_value);
                self.pc += 2;
            }

            Opcode::ADDRESS => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                let address_bytes = self.address.as_bytes();
                let mut value = [0u8; 8];
                value.copy_from_slice(&address_bytes[..8]);
                self.registers.set(dst, u64::from_le_bytes(value));
                self.pc += 2;
            }

            Opcode::BLOCKNUMBER => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers.set(dst, self.block_number);
                self.pc += 2;
            }

            Opcode::TIMESTAMP => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers.set(dst, self.timestamp);
                self.pc += 2;
            }

            Opcode::GAS => {
                self.gas.consume(GasCosts::BASE)?;
                let dst = self.decode_r();
                self.registers.set(dst, self.gas.remaining());
                self.pc += 2;
            }
        }

        Ok(())
    }

    /// Decode a single register operand: [opcode, RRRR____]
    fn decode_r(&self) -> usize {
        ((self.bytecode[self.pc + 1] >> 4) & 0x0F) as usize
    }

    /// Decode two register operands: [opcode, RRRR_SSSS]
    fn decode_rr(&self) -> (usize, usize) {
        let byte = self.bytecode[self.pc + 1];
        let r1 = ((byte >> 4) & 0x0F) as usize;
        let r2 = (byte & 0x0F) as usize;
        (r1, r2)
    }

    /// Decode three register operands: [opcode, DDDD_SSS1, SSS2____]
    fn decode_rrr(&self) -> (usize, usize, usize) {
        let b1 = self.bytecode[self.pc + 1];
        let b2 = self.bytecode[self.pc + 2];
        let dst = ((b1 >> 4) & 0x0F) as usize;
        let s1 = (b1 & 0x0F) as usize;
        let s2 = ((b2 >> 4) & 0x0F) as usize;
        (dst, s1, s2)
    }

    /// Decode a 64-bit immediate value (little-endian).
    fn decode_imm64(&self) -> u64 {
        let start = self.pc + 2;
        let bytes = &self.bytecode[start..start + 8];
        u64::from_le_bytes(bytes.try_into().unwrap())
    }

    /// Decode a 32-bit immediate value (little-endian).
    fn decode_imm32(&self) -> u32 {
        let start = self.pc + 2;
        let bytes = &self.bytecode[start..start + 4];
        u32::from_le_bytes(bytes.try_into().unwrap())
    }
}

/// Storage interface for the VM.
pub trait StorageBackend {
    /// Read 32 bytes from storage slot.
    fn sload(&self, key: &[u8; 32]) -> [u8; 32];

    /// Write 32 bytes to storage slot.
    fn sstore(&mut self, key: &[u8; 32], value: &[u8; 32]);
}

impl Vm {
    /// Execute SLOAD: read from persistent storage.
    fn execute_sload(&mut self) -> Result<(), VmError> {
        self.gas.consume(GasCosts::SLOAD)?;

        let (dst, key_reg) = self.decode_rr();

        let key_value = self.registers.get(key_reg);
        let mut key = [0u8; 32];
        key[24..32].copy_from_slice(&key_value.to_be_bytes());

        let value = if let Some(storage) = &self.storage {
            storage.sload(&key)
        } else {
            [0u8; 32]
        };

        self.registers
            .set(dst, u64::from_be_bytes(value[24..32].try_into().unwrap()));

        self.pc += 2;
        Ok(())
    }

    /// Execute SSTORE: write to persistent storage.
    fn execute_sstore_internal(&mut self) -> Result<(), VmError> {
        let (key_reg, value_reg) = self.decode_rr();

        let key_value = self.registers.get(key_reg);
        let mut key = [0u8; 32];
        key[24..32].copy_from_slice(&key_value.to_be_bytes());

        let cost = if let Some(storage) = &self.storage {
            let current = storage.sload(&key);
            let is_empty = current == [0u8; 32];
            if is_empty {
                GasCosts::SSTORE_SET
            } else {
                GasCosts::SSTORE_RESET
            }
        } else {
            GasCosts::SSTORE_SET
        };
        self.gas.consume(cost)?;

        let mut value = [0u8; 32];
        value[24..32].copy_from_slice(&self.registers.get(value_reg).to_be_bytes());

        if let Some(storage) = &mut self.storage {
            storage.sstore(&key, &value);
        }

        self.pc += 2;
        Ok(())
    }
}
