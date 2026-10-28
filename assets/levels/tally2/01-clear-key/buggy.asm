; Tally-2 firmware 0.9
;
; M1-M2  count, M2 is the rightmost digit
; M3     last key

reset:  LDI  A, 0
        ST   M1
        ST   M2
main:   WAIT
        IN   KEY
        ST   M3
        CPI  12
        JZ   reset
        CPI  10
        JZ   count
        JMP  main

count:  LD   M2
        INC
        CPI  10
        JC   ones
        LDI  A, 0
        ST   M2
        JMP  tens

ones:   ST   M2
        JMP  main

tens:   LD   M1
        INC
        CPI  10
        JC   tstore
        LDI  A, 0
tstore: ST   M1
        JMP  main
