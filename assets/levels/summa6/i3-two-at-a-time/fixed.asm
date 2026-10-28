; Summa-6 firmware 0.3
;
; M1-M6  display, M6 is the rightmost digit
; M7     always 1
; M8     always 0

reset:  LDI  A, 1
        ST   M7
        LDI  A, 0
        ST   M8
        LDI  B, 6
wipe:   ST   [B]
        DJNZ wipe
main:   WAIT
        IN   KEY
        CPI  10             ; only the + key counts
        JNZ  main
        CLC                 ; no carry yet
        LD   M6             ; right digit + 1 + carry
        ADC  M7
        DA
        ST   M6
        LDI  B, 5
up:     LD   [B]            ; carry 1 into the next digit
        ADC  M8
        DA
        ST   [B]
        DJNZ up
        JMP  main
