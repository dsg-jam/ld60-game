; Lock-4 firmware 1.0
;
; M1-M4  display and entry, M4 is the rightmost digit
; M5-M8  door code
; M9     0 shut, 1 open, 2 new code
; MA     wrong codes in a row
; MB     1 if the next digit starts a new entry
; MC     scratch

reset:  LDI  A, 0
        LDI  B, 4
rclr:   ST   [B+4]
        DJNZ rclr
        ST   MA
clrkey: LDI  A, 0
        ST   M9
        ST   MA
        LDI  A, 2
        OUT  ANN
        CALL blank
main:   WAIT
        IN   KEY
        CPI  10
        JC   digit
        CPI  11
        JZ   newkey
        CPI  12
        JZ   clrkey
enter:  LDI  A, 1
        ST   MB
        LD   M9
        CPI  2
        JZ   store
        LDI  B, 4
cmp:    LD   [B]
        CMP  [B+4]
        JNZ  bad
        DJNZ cmp
        LDI  A, 0
        ST   MA
        LDI  A, 1
        ST   M9
        LDI  A, 1
        OUT  ANN
        JMP  main
bad:    LDI  A, 0
        ST   M9
        LDI  A, 2
        OUT  ANN
        LD   MA
        INC
        ST   MA
        CPI  3
        JC   main
        LDI  A, 6
        OUT  ANN
jam:    WAIT
        IN   KEY
        JMP  jam

store:  LDI  B, 4
copy:   LD   [B]
        ST   [B+4]
        DJNZ copy
        JMP  clrkey

newkey: LD   M9
        CPI  1
        JNZ  main
        LDI  A, 2
        ST   M9
        LDI  A, 3
        OUT  ANN
        CALL blank
        JMP  main

digit:  ST   MC
        LD   MB
        CPI  0
        JZ   dload
        CALL blank
dload:  LD   M2
        ST   M1
        LD   M3
        ST   M2
        LD   M4
        ST   M3
        LD   MC
        ST   M4
        JMP  main

blank:  LDI  A, 0
        LDI  B, 4
bclr:   ST   [B]
        DJNZ bclr
        ST   MB
        RET
