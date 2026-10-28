; Tally-2 firmware 0.1
;
; M1-M2  count, M2 is the rightmost digit

start:  LDI  A, 0           ; put 0 in A
        ST   M1             ; show it on the left digit
        LDI  A, 0           ; put 0 in A
        ST   M2             ; show it on the right digit
count:  WAIT                ; wait for a key press
        LD   M2             ; one more
        INC
        ST   M2
        JMP  count
