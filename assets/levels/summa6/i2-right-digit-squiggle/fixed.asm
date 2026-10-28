; Summa-6 firmware 0.2
;
; M1-M6  display, M6 is the rightmost digit

reset:  CALL clear
main:   WAIT
        IN   KEY
        CPI  10
        JNC  main
        ST   M6
        JMP  main

clear:  LDI  A, 0
        LDI  B, 6           ; 6 digits to clear
wipe:   ST   [B]            ; digit B is 0, B counts down
        DJNZ wipe
        RET
