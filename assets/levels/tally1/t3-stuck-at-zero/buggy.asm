; Tally-2 firmware 0.3
;
; M1-M2  count, M2 is the rightmost digit

start:  LDI  A, 0
        ST   M1
        ST   M2             ; the display shows 00
count:  WAIT
        LD   M2
        INC
        ST   M2
        JMP  start          ; wait for the next press
