; Tally-1 firmware 0.2
;
; M2     count

start:  LDI  A, 0
        ST   M2
count:  WAIT                ; wait for a key press
        LD   M2             ; A = the digit, plus one
        INC
        CPI  10
        JC   show
        LDI  A, 0           ; after 9 comes 0
show:   ST   M2             ; show it
        JMP  count          ; back up to count
