; Tally-1 firmware 0.1
;
; M2     count

start:  LDI  A, 0           ; put 0 in A
        ST   M2             ; show it
count:  WAIT                ; wait for a key press
        LD   M2             ; one more
        INC
        CPI  10
        JC   show
        LDI  A, 0           ; after 9 comes 0
show:   ST   M2
        JMP  count
