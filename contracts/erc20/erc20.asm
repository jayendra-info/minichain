;=============================================================================
; ERC20 Token Contract for Minichain
;=============================================================================
;
; Standard ERC20 implementation with mint and burn functionality.
;
; Storage Layout:
;   Slot 0: total_supply (u64)
;   Slot 1: owner_address (u64)
;   Slot hash(address || 0): balances[address]
;   Slot hash(address || hash(spender || 1)): allowances[owner][spender]
;
; Function Selectors (first 8 bytes of calldata):
;   0x00: totalSupply()
;   0x01: balanceOf(address)
;   0x02: transfer(to, amount)
;   0x03: approve(spender, amount)
;   0x04: transferFrom(from, to, amount)
;   0x05: allowance(owner, spender)
;   0x06: mint(to, amount)
;   0x07: burn(amount)
;
;=============================================================================

.entry main

;=============================================================================
; MAIN DISPATCHER
;=============================================================================

main:
    ; Get function selector from calldata (first 8 bytes loaded to memory)
    LOADI R15, 0                 ; memory offset 0
    LOAD64 R0, R15               ; load function ID from calldata into R0

    ; Route to appropriate function based on function ID
    ; Total supply
    LOADI R1, 0
    EQ R2, R0, R1
    LOADI R3, func_totalSupply
    JUMPI R2, R3

    ; balanceOf
    LOADI R1, 1
    EQ R2, R0, R1
    LOADI R3, func_balanceOf
    JUMPI R2, R3

    ; transfer
    LOADI R1, 2
    EQ R2, R0, R1
    LOADI R3, func_transfer
    JUMPI R2, R3

    ; approve
    LOADI R1, 3
    EQ R2, R0, R1
    LOADI R3, func_approve
    JUMPI R2, R3

    ; transferFrom
    LOADI R1, 4
    EQ R2, R0, R1
    LOADI R3, func_transferFrom
    JUMPI R2, R3

    ; allowance
    LOADI R1, 5
    EQ R2, R0, R1
    LOADI R3, func_allowance
    JUMPI R2, R3

    ; mint
    LOADI R1, 6
    EQ R2, R0, R1
    LOADI R3, func_mint
    JUMPI R2, R3

    ; burn
    LOADI R1, 7
    EQ R2, R0, R1
    LOADI R3, func_burn
    JUMPI R2, R3

    ; Unknown function - revert
    REVERT

;=============================================================================
; FUNCTION: totalSupply()
; Returns the total supply from storage slot 0
;=============================================================================
func_totalSupply:
    LOADI R0, 0                  ; Storage slot 0 = total_supply
    SLOAD R1, R0                 ; Load total_supply into R1
    LOADI R2, 0                  ; Memory offset 0
    STORE64 R2, R1               ; Store return value in memory[0]
    HALT

;=============================================================================
; FUNCTION: balanceOf(address)
; Gets balance of address from storage
;=============================================================================
func_balanceOf:
    LOADI R0, 8
    LOAD64 R1, R0                ; Load address parameter from calldata[8]
    XOR R3, R1, R1               ; Clear a temp register (XOR with itself = 0)
    XOR R3, R1, R3               ; R3 = address XOR 0 = address (balance key)
    SLOAD R4, R3                 ; Load balance from storage
    LOADI R5, 0
    STORE64 R5, R4               ; Store return value
    HALT

;=============================================================================
; FUNCTION: transfer(to, amount)
; Transfer tokens from caller to recipient
;=============================================================================
func_transfer:
    ; Load parameters
    CALLER R1                    ; R1 = sender (msg.sender)
    LOADI R0, 8
    LOAD64 R2, R0                ; R2 = to address
    LOADI R0, 16
    LOAD64 R3, R0                ; R3 = amount

    ; Validation: amount > 0
    LOADI R4, 0
    LE R5, R3, R4                ; if amount <= 0, jump to revert
    LOADI R6, transfer_revert
    JUMPI R5, R6

    ; Get sender balance
    XOR R4, R1, R4               ; R4 = sender XOR 0 = sender (balance key)
    SLOAD R5, R4                 ; R5 = sender balance

    ; Check balance >= amount
    LT R6, R5, R3                ; if balance < amount
    LOADI R7, transfer_revert
    JUMPI R6, R7

    ; Update sender balance
    SUB R7, R5, R3               ; R7 = new sender balance
    SSTORE R4, R7                ; Store updated balance

    ; Update recipient balance
    XOR R8, R2, R8               ; R8 = recipient XOR 0 = recipient
    SLOAD R9, R8                 ; R9 = recipient balance
    ADD R10, R9, R3              ; R10 = new recipient balance
    SSTORE R8, R10               ; Store updated balance

    ; Return success
    LOADI R11, 0
    LOADI R12, 1
    STORE64 R11, R12
    HALT

transfer_revert:
    REVERT

;=============================================================================
; FUNCTION: approve(spender, amount)
; Set allowance for spender
;=============================================================================
func_approve:
    CALLER R1                    ; R1 = owner (msg.sender)
    LOADI R0, 8
    LOAD64 R2, R0                ; R2 = spender
    LOADI R0, 16
    LOAD64 R3, R0                ; R3 = amount

    ; Compute storage key: owner XOR (spender XOR 1)
    LOADI R4, 1                  ; R4 = 1
    XOR R5, R2, R4               ; R5 = spender XOR 1
    XOR R6, R1, R5               ; R6 = owner XOR (spender XOR 1)

    ; Store allowance
    SSTORE R6, R3

    ; Return success
    LOADI R7, 0
    LOADI R8, 1
    STORE64 R7, R8
    HALT

;=============================================================================
; FUNCTION: transferFrom(from, to, amount)
; Transfer using allowance
;=============================================================================
func_transferFrom:
    CALLER R1                    ; R1 = spender
    LOADI R0, 8
    LOAD64 R2, R0                ; R2 = from
    LOADI R0, 16
    LOAD64 R3, R0                ; R3 = to
    LOADI R0, 24
    LOAD64 R4, R0                ; R4 = amount

    ; Check amount > 0
    LOADI R5, 0
    LE R6, R4, R5
    LOADI R7, transferFrom_revert
    JUMPI R6, R7

    ; Check allowance
    LOADI R5, 1
    XOR R6, R1, R5               ; R6 = spender XOR 1
    XOR R7, R2, R6               ; R7 = from XOR (spender XOR 1)
    SLOAD R8, R7                 ; R8 = allowance
    LT R9, R8, R4
    LOADI R10, transferFrom_revert
    JUMPI R9, R10

    ; Check from balance
    XOR R10, R2, R10             ; R10 = from balance key
    SLOAD R11, R10               ; R11 = from balance
    LT R12, R11, R4
    LOADI R13, transferFrom_revert
    JUMPI R12, R13

    ; Update allowance
    SUB R13, R8, R4
    SSTORE R7, R13

    ; Update from balance
    SUB R14, R11, R4
    SSTORE R10, R14

    ; Update to balance
    XOR R15, R3, R15             ; R15 = to balance key
    SLOAD R0, R15
    ADD R1, R0, R4
    SSTORE R15, R1

    ; Return success
    LOADI R2, 0
    LOADI R3, 1
    STORE64 R2, R3
    HALT

transferFrom_revert:
    REVERT

;=============================================================================
; FUNCTION: allowance(owner, spender)
; Return approved amount
;=============================================================================
func_allowance:
    LOADI R0, 8
    LOAD64 R1, R0                ; R1 = owner
    LOADI R0, 16
    LOAD64 R2, R0                ; R2 = spender

    ; Compute key
    LOADI R3, 1
    XOR R4, R2, R3
    XOR R5, R1, R4

    SLOAD R6, R5
    LOADI R7, 0
    STORE64 R7, R6
    HALT

;=============================================================================
; FUNCTION: mint(to, amount)
; Create new tokens (owner only)
;=============================================================================
func_mint:
    ; Check caller is owner
    CALLER R1
    LOADI R2, 1                  ; Storage slot 1 = owner
    SLOAD R3, R2
    NE R4, R1, R3
    LOADI R5, mint_revert
    JUMPI R4, R5

    ; Get parameters
    LOADI R0, 8
    LOAD64 R2, R0                ; R2 = to
    LOADI R0, 16
    LOAD64 R5, R0                ; R5 = amount

    ; Check amount > 0
    LOADI R6, 0
    LE R7, R5, R6
    LOADI R8, mint_revert
    JUMPI R7, R8

    ; Update total supply
    LOADI R0, 0
    SLOAD R7, R0
    ADD R8, R7, R5
    SSTORE R0, R8

    ; Update recipient balance
    XOR R9, R2, R9               ; R9 = to balance key
    SLOAD R10, R9
    ADD R11, R10, R5
    SSTORE R9, R11

    ; Return success
    LOADI R12, 0
    LOADI R13, 1
    STORE64 R12, R13
    HALT

mint_revert:
    REVERT

;=============================================================================
; FUNCTION: burn(amount)
; Destroy tokens
;=============================================================================
func_burn:
    CALLER R1                    ; R1 = caller
    LOADI R0, 8
    LOAD64 R2, R0                ; R2 = amount

    ; Check amount > 0
    LOADI R3, 0
    LE R4, R2, R3
    LOADI R5, burn_revert
    JUMPI R4, R5

    ; Check balance
    XOR R5, R1, R5               ; R5 = balance key
    SLOAD R6, R5
    LT R7, R6, R2
    LOADI R8, burn_revert
    JUMPI R7, R8

    ; Update balance
    SUB R8, R6, R2
    SSTORE R5, R8

    ; Update total supply
    LOADI R0, 0
    SLOAD R9, R0
    SUB R10, R9, R2
    SSTORE R0, R10

    ; Return success
    LOADI R11, 0
    LOADI R12, 1
    STORE64 R11, R12
    HALT

burn_revert:
    REVERT

;=============================================================================
; END OF ERC20 CONTRACT
;=============================================================================
