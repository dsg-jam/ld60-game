; Tally-2 firmware 0.5
;
; M1-M2  count, M2 is the rightmost digit

start:  LDI  A, 0
        ST   M1
        ST   M2
count:  WAIT
        IN   KEY            ; A = code of the key
        LD   M2
        INC
        ST   M2
        JMP  count
