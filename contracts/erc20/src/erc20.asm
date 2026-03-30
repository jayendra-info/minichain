;=============================================================================
; ERC20 Token Contract for Minichain
;=============================================================================
;
; Calldata layout: 8-byte little-endian selector followed by u64 arguments.
;
; Selectors:
;   0x00 totalSupply()
;   0x01 balanceOf(address_id)
;   0x02 transfer(to, amount)
;   0x03 approve(spender, amount)
;   0x04 transferFrom(from, to, amount)
;   0x05 allowance(owner, spender)
;   0x06 mint(to, amount)
;   0x07 burn(amount)
;   0x08 name()
;   0x09 symbol()
;   0x0A decimals()
;   0xFF init(owner, name, symbol, decimals, initial_to, initial_supply)
;
; Storage layout:
;   Slot 0: total supply
;   Slot 1: owner
;   Slot 2: name (up to 8 ASCII chars encoded as u64)
;   Slot 3: symbol (up to 8 ASCII chars encoded as u64)
;   Slot 4: decimals
;   balance[address]   = address XOR 0x1000000000000000
;   allowance[a][b]    = a XOR b XOR 0x2000000000000000
;=============================================================================

.entry main

main:
    LOADI R15, 0
    LOAD64 R0, R15

    LOADI R1, 0
    EQ R2, R0, R1
    LOADI R3, func_totalSupply
    JUMPI R2, R3

    LOADI R1, 1
    EQ R2, R0, R1
    LOADI R3, func_balanceOf
    JUMPI R2, R3

    LOADI R1, 2
    EQ R2, R0, R1
    LOADI R3, func_transfer
    JUMPI R2, R3

    LOADI R1, 3
    EQ R2, R0, R1
    LOADI R3, func_approve
    JUMPI R2, R3

    LOADI R1, 4
    EQ R2, R0, R1
    LOADI R3, func_transferFrom
    JUMPI R2, R3

    LOADI R1, 5
    EQ R2, R0, R1
    LOADI R3, func_allowance
    JUMPI R2, R3

    LOADI R1, 6
    EQ R2, R0, R1
    LOADI R3, func_mint
    JUMPI R2, R3

    LOADI R1, 7
    EQ R2, R0, R1
    LOADI R3, func_burn
    JUMPI R2, R3

    LOADI R1, 8
    EQ R2, R0, R1
    LOADI R3, func_name
    JUMPI R2, R3

    LOADI R1, 9
    EQ R2, R0, R1
    LOADI R3, func_symbol
    JUMPI R2, R3

    LOADI R1, 10
    EQ R2, R0, R1
    LOADI R3, func_decimals
    JUMPI R2, R3

    LOADI R1, 255
    EQ R2, R0, R1
    LOADI R3, func_init
    JUMPI R2, R3

    REVERT

func_totalSupply:
    LOADI R0, 0
    SLOAD R1, R0
    LOADI R2, 0
    STORE64 R2, R1
    HALT

func_balanceOf:
    LOADI R0, 8
    LOAD64 R1, R0
    LOADI R2, 1152921504606846976 ; 0x1000000000000000
    XOR R3, R1, R2
    SLOAD R4, R3
    LOADI R5, 0
    STORE64 R5, R4
    HALT

func_transfer:
    CALLER R1
    LOADI R0, 8
    LOAD64 R2, R0
    LOADI R0, 16
    LOAD64 R3, R0

    LOADI R4, 0
    LE R5, R3, R4
    LOADI R6, revert_transfer
    JUMPI R5, R6

    LOADI R4, 1152921504606846976
    XOR R5, R1, R4
    SLOAD R6, R5
    LT R7, R6, R3
    LOADI R8, revert_transfer
    JUMPI R7, R8

    SUB R9, R6, R3
    SSTORE R5, R9

    XOR R10, R2, R4
    SLOAD R11, R10
    ADD R12, R11, R3
    SSTORE R10, R12

    LOADI R13, 0
    LOADI R14, 1
    STORE64 R13, R14
    HALT

func_approve:
    CALLER R1
    LOADI R0, 8
    LOAD64 R2, R0
    LOADI R0, 16
    LOAD64 R3, R0

    LOADI R4, 2305843009213693952 ; 0x2000000000000000
    XOR R5, R1, R2
    XOR R6, R5, R4
    SSTORE R6, R3

    LOADI R7, 0
    LOADI R8, 1
    STORE64 R7, R8
    HALT

func_transferFrom:
    CALLER R1
    LOADI R0, 8
    LOAD64 R2, R0
    LOADI R0, 16
    LOAD64 R3, R0
    LOADI R0, 24
    LOAD64 R4, R0

    LOADI R5, 0
    LE R6, R4, R5
    LOADI R7, revert_transfer_from
    JUMPI R6, R7

    LOADI R5, 2305843009213693952
    XOR R6, R2, R1
    XOR R7, R6, R5
    SLOAD R8, R7
    LT R9, R8, R4
    LOADI R10, revert_transfer_from
    JUMPI R9, R10

    LOADI R5, 1152921504606846976
    XOR R10, R2, R5
    SLOAD R11, R10
    LT R12, R11, R4
    LOADI R13, revert_transfer_from
    JUMPI R12, R13

    SUB R13, R8, R4
    SSTORE R7, R13

    SUB R14, R11, R4
    SSTORE R10, R14

    XOR R15, R3, R5
    SLOAD R0, R15
    ADD R1, R0, R4
    SSTORE R15, R1

    LOADI R2, 0
    LOADI R3, 1
    STORE64 R2, R3
    HALT

func_allowance:
    LOADI R0, 8
    LOAD64 R1, R0
    LOADI R0, 16
    LOAD64 R2, R0
    LOADI R3, 2305843009213693952
    XOR R4, R1, R2
    XOR R5, R4, R3
    SLOAD R6, R5
    LOADI R7, 0
    STORE64 R7, R6
    HALT

func_mint:
    CALLER R1
    LOADI R2, 1
    SLOAD R3, R2
    NE R4, R1, R3
    LOADI R5, revert_mint
    JUMPI R4, R5

    LOADI R0, 8
    LOAD64 R2, R0
    LOADI R0, 16
    LOAD64 R5, R0

    LOADI R6, 0
    LE R7, R5, R6
    LOADI R8, revert_mint
    JUMPI R7, R8

    LOADI R0, 0
    SLOAD R7, R0
    ADD R8, R7, R5
    SSTORE R0, R8

    LOADI R9, 1152921504606846976
    XOR R10, R2, R9
    SLOAD R11, R10
    ADD R12, R11, R5
    SSTORE R10, R12

    LOADI R13, 0
    LOADI R14, 1
    STORE64 R13, R14
    HALT

func_burn:
    CALLER R1
    LOADI R0, 8
    LOAD64 R2, R0

    LOADI R3, 0
    LE R4, R2, R3
    LOADI R5, revert_burn
    JUMPI R4, R5

    LOADI R6, 1152921504606846976
    XOR R7, R1, R6
    SLOAD R8, R7
    LT R9, R8, R2
    LOADI R10, revert_burn
    JUMPI R9, R10

    SUB R11, R8, R2
    SSTORE R7, R11

    LOADI R12, 0
    SLOAD R13, R12
    SUB R14, R13, R2
    SSTORE R12, R14

    LOADI R15, 0
    LOADI R0, 1
    STORE64 R15, R0
    HALT

func_name:
    LOADI R0, 2
    SLOAD R1, R0
    LOADI R2, 0
    STORE64 R2, R1
    HALT

func_symbol:
    LOADI R0, 3
    SLOAD R1, R0
    LOADI R2, 0
    STORE64 R2, R1
    HALT

func_decimals:
    LOADI R0, 4
    SLOAD R1, R0
    LOADI R2, 0
    STORE64 R2, R1
    HALT

func_init:
    LOADI R0, 1
    SLOAD R1, R0
    LOADI R2, 0
    NE R3, R1, R2
    LOADI R4, revert_init
    JUMPI R3, R4

    LOADI R5, 8
    LOAD64 R6, R5              ; owner
    LOADI R5, 16
    LOAD64 R7, R5              ; name
    LOADI R5, 24
    LOAD64 R8, R5              ; symbol
    LOADI R5, 32
    LOAD64 R9, R5              ; decimals
    LOADI R5, 40
    LOAD64 R10, R5             ; initial recipient
    LOADI R5, 48
    LOAD64 R11, R5             ; initial supply

    LOADI R0, 1
    SSTORE R0, R6
    LOADI R0, 2
    SSTORE R0, R7
    LOADI R0, 3
    SSTORE R0, R8
    LOADI R0, 4
    SSTORE R0, R9

    LOADI R12, 0
    LE R13, R11, R12
    LOADI R14, finish_init
    JUMPI R13, R14

    LOADI R0, 0
    SSTORE R0, R11
    LOADI R15, 1152921504606846976
    XOR R0, R10, R15
    SSTORE R0, R11

finish_init:
    LOADI R1, 0
    LOADI R2, 1
    STORE64 R1, R2
    HALT

revert_transfer:
    REVERT

revert_transfer_from:
    REVERT

revert_mint:
    REVERT

revert_burn:
    REVERT

revert_init:
    REVERT
