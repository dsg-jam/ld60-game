; Summa-6 firmware 0.1
;
; M1-M6  display, M6 is the rightmost digit

reset:  CALL clear          ; run clear, then come back here
main:   WAIT
        IN   KEY            ; A = code of the key
        CPI  10             ; only the digit keys
        JNC  main
        ST   M6             ; show it on the right digit
        JMP  main

clear:  LDI  A, 0           ; set every digit to 0
        ST   M1
        ST   M2
        ST   M3
        ST   M4
        ST   M5
        ST   M6
        RET
