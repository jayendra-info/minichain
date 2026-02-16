//! Gas metering.

use crate::executor::VmError;

/// Gas costs for operations.
pub struct GasCosts;

impl GasCosts {
    // Tier 1: Very cheap (simple register operations)
    pub const ZERO: u64 = 0; // HALT, NOP
    pub const BASE: u64 = 2; // ADD, SUB, AND, OR, MOV

    // Tier 2: Cheap (more complex ALU operations)
    pub const LOW: u64 = 3; // MUL, comparison ops

    // Tier 3: Medium (division, shifts)
    pub const MID: u64 = 5; // DIV, MOD, SHL, SHR

    // Tier 4: Memory operations
    pub const MEMORY_READ: u64 = 3;
    pub const MEMORY_WRITE: u64 = 3;
    pub const MEMORY_GROW_PER_BYTE: u64 = 1;

    // Tier 5: Storage (expensive!)
    pub const SLOAD: u64 = 100; // Read from storage
    pub const SSTORE_SET: u64 = 20000; // Write to empty slot
    pub const SSTORE_RESET: u64 = 5000; // Overwrite existing slot

    // Control flow
    pub const JUMP: u64 = 8;
    pub const CALL: u64 = 700;
}

/// Gas meter tracks remaining gas.
pub struct GasMeter {
    remaining: u64,
    used: u64,
}

impl GasMeter {
    pub fn new(limit: u64) -> Self {
        Self {
            remaining: limit,
            used: 0,
        }
    }

    /// Consume gas, returning error if insufficient.
    pub fn consume(&mut self, amount: u64) -> Result<(), VmError> {
        if self.remaining < amount {
            return Err(VmError::OutOfGas {
                required: amount,
                remaining: self.remaining,
            });
        }
        self.remaining -= amount;
        self.used += amount;
        Ok(())
    }

    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    pub fn used(&self) -> u64 {
        self.used
    }

    /// Calculate gas for memory expansion.
    pub fn memory_expansion_cost(current_size: usize, new_size: usize) -> u64 {
        if new_size <= current_size {
            return 0;
        }
        let expansion = (new_size - current_size) as u64;
        expansion * GasCosts::MEMORY_GROW_PER_BYTE
    }
}
