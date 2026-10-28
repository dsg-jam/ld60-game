; Tally-2 firmware 0.6
;
; M1-M2  count, M2 is the rightmost digit

start:  LDI  A, 0
        ST   M1
        ST   M2
count:  WAIT
        IN   KEY
        CPI  13
        JZ   start
        LD   M2             ; one more on the right digit
        INC
        CPI  10             ; lower than 10: just show it
        JC   ones
        LDI  A, 0           ; else the right digit starts at 0
        ST   M2
        LD   M1             ; and the left digit goes up
        INC
        ST   M1
        JMP  count

ones:   ST   M2
        JMP  count
