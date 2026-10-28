; Tally-1 firmware 0.3
;
; M2     count

start:  LDI  A, 0
        ST   M2             ; the display shows 0
count:  WAIT
        LD   M2
        INC
        CPI  10
        JC   show
        LDI  A, 0           ; after 9 comes 0
show:   ST   M2
        JMP  count          ; wait for the next press
