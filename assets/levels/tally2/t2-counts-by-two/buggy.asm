; Tally-2 firmware 0.2
;
; M1-M2  count, M2 is the rightmost digit

start:  LDI  A, 0
        ST   M1
        ST   M2
count:  WAIT                ; wait for a key press
        LD   M2             ; A = right digit, plus one
        INC
        INC                 ; and one more
        ST   M2             ; show it
        JMP  count          ; back up to count
