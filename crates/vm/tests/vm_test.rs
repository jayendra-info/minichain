use minichain_core::crypto::Address;
use minichain_vm::Vm;

#[test]
fn test_add() {
    // LOADI R0, 10
    // LOADI R1, 20
    // ADD R2, R0, R1
    // LOG R2
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x10, 0x14, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0x10, 0xF0, 0x20, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![30]);
}

#[test]
fn test_sub() {
    // LOADI R0, 20
    // LOADI R1, 8
    // SUB R2, R0, R1
    // LOG R2
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x10, 0x08, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x20, 0x10, 0xF0, 0x20, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![12]);
}

#[test]
fn test_mul() {
    // LOADI R0, 6
    // LOADI R1, 7
    // MUL R2, R0, R1
    // LOG R2
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x10, 0x07, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x20, 0x10, 0xF0, 0x20, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![42]);
}

#[test]
fn test_div() {
    // LOADI R0, 20
    // LOADI R1, 4
    // DIV R2, R0, R1
    // LOG R2
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x10, 0x04, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x13, 0x20, 0x10, 0xF0, 0x20, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![5]);
}

#[test]
fn test_bitwise_and() {
    // LOADI R0, 0xFF
    // LOADI R1, 0x0F
    // AND R2, R0, R1
    // LOG R2
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x10, 0x0F, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x20, 0x10, 0xF0, 0x20, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![0x0F]);
}

#[test]
fn test_comparison_lt() {
    // LOADI R0, 5
    // LOADI R1, 10
    // LT R2, R0, R1  (5 < 10 = true = 1)
    // LOG R2
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x10, 0x0A, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x32, 0x20, 0x10, 0xF0, 0x20, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![1]);
}

#[test]
fn test_mov() {
    // LOADI R0, 42
    // MOV R1, R0  (0x71 = opcode, 0x10 = dst:R1 src:R0)
    // LOG R1
    // HALT
    let bytecode = vec![
        0x70, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x71, 0x10, 0xF0, 0x10, 0x00,
    ];

    let mut vm = Vm::new(bytecode, 1_000_000, Address::ZERO, Address::ZERO, 0);
    let result = vm.run().unwrap();

    assert!(result.success);
    assert_eq!(result.logs, vec![42]);
}

#[test]
fn test_out_of_gas() {
    // Simple program that should run out of gas
    let bytecode = vec![
        0x70, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    // Gas limit of 1 is not enough for LOADI (costs 2)
    let mut vm = Vm::new(bytecode, 1, Address::ZERO, Address::ZERO, 0);
    let result = vm.run();

    assert!(result.is_err());
}
