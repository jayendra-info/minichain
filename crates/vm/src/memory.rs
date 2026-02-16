//! VM memory model.

use crate::executor::VmError;

pub const NUM_REGISTERS: usize = 16;

pub struct Registers {
    values: [u64; NUM_REGISTERS],
}

impl Registers {
    pub fn new() -> Self {
        Self {
            values: [0; NUM_REGISTERS],
        }
    }

    pub fn get(&self, r: usize) -> u64 {
        self.values[r % NUM_REGISTERS]
    }

    pub fn set(&mut self, r: usize, value: u64) {
        self.values[r % NUM_REGISTERS] = value;
    }

    pub fn values(&self) -> &[u64; NUM_REGISTERS] {
        &self.values
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Memory {
    data: Vec<u8>,
    max_size: usize,
}

impl Memory {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Vec::new(),
            max_size,
        }
    }

    pub fn load8(&self, offset: u32) -> u8 {
        self.data.get(offset as usize).copied().unwrap_or(0)
    }

    pub fn load64(&self, offset: u32) -> u64 {
        if offset as usize + 8 > self.data.len() {
            return 0;
        }
        let bytes = &self.data[offset as usize..offset as usize + 8];
        u64::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn store8(&mut self, offset: u32, value: u8) -> Result<(), VmError> {
        let offset = offset as usize;
        if offset >= self.max_size {
            return Err(VmError::MemoryOverflow);
        }
        if offset >= self.data.len() {
            self.data.resize(offset + 1, 0);
        }
        self.data[offset] = value;
        Ok(())
    }

    pub fn store64(&mut self, offset: u32, value: u64) -> Result<(), VmError> {
        let offset = offset as usize;
        if offset + 8 > self.max_size {
            return Err(VmError::MemoryOverflow);
        }
        if offset + 8 > self.data.len() {
            self.data.resize(offset + 8, 0);
        }
        self.data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn mcopy(&mut self, dest: u32, src: u32, length: u32) -> Result<(), VmError> {
        let dest = dest as usize;
        let src = src as usize;
        let length = length as usize;

        if dest + length > self.max_size || src + length > self.data.len() {
            return Err(VmError::MemoryOverflow);
        }

        if dest >= self.data.len() {
            self.data.resize(dest, 0);
        }
        if dest + length > self.data.len() {
            self.data.resize(dest + length, 0);
        }

        let src_data = self.data[src..src + length].to_vec();
        self.data[dest..dest + length].copy_from_slice(&src_data);
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}
