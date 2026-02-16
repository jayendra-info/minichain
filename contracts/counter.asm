.entry main

main:
    LOADI R0, 0          ; storage slot 0 = counter
    SLOAD R1, R0         ; load current value
    LOADI R2, 1
    ADD R1, R1, R2       ; increment
    SSTORE R0, R1        ; save back
    HALT
