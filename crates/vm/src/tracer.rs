//! Execution tracer.

use crate::opcodes::Opcode;

/// A single trace entry.
#[derive(Debug, Clone)]
pub struct TraceStep {
    pub pc: usize,
    pub opcode: Opcode,
    pub gas_before: u64,
    pub gas_after: u64,
    pub registers: [u64; 16],
}

/// Records execution for debugging.
pub struct Tracer {
    steps: Vec<TraceStep>,
    enabled: bool,
}

impl Tracer {
    pub fn new(enabled: bool) -> Self {
        Self {
            steps: Vec::new(),
            enabled,
        }
    }

    pub fn record(&mut self, step: TraceStep) {
        if self.enabled {
            self.steps.push(step);
        }
    }

    pub fn steps(&self) -> &[TraceStep] {
        &self.steps
    }

    /// Print a human-readable trace.
    pub fn print_trace(&self) {
        for (i, step) in self.steps.iter().enumerate() {
            println!(
                "{:4}: PC={:04X} {:?} gas={} R0={} R1={} R2={}",
                i,
                step.pc,
                step.opcode,
                step.gas_after,
                step.registers[0],
                step.registers[1],
                step.registers[2],
            );
        }
    }
}
