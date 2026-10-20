; Tally-2 firmware 1.0
;
; M1-M2  count, M2 is the rightmost digit
; M3     last key

reset:  LDI  A, 0
        ST   M1
        ST   M2
main:   HLT
        IN   KEY
        ST   M3
        CPI  0x0D
        JZ   reset
        CPI  0x0A
        JZ   count
        JMP  main

count:  LD   M2
        INC
        CPI  10
        JC   ones
        LDI  A, 0
        ST   M2
tens:   LD   M1
        INC
        ST   M1
        JMP  main

ones:   ST   M2
        JMP  main
