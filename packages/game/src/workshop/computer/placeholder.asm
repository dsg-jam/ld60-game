; Adding machine firmware for CalCo rev. A
; M1-M6  entry and display, most significant digit first
; M9-ME  running total

        .org 0x00
reset:  CALL clear

main:   HLT                 ; wait for a key
        IN   KEY
        CPI  0x0A
        JC   digit          ; 0 to 9
        CPI  0x0D
        JZ   reset          ; C clears everything
        CALL add            ; + and = add the entry
        CALL show
        JMP  main

digit:  ST   M0             ; shift the entry left
        LD   M2
        ST   M1
        LD   M3
        ST   M2
        LD   M4
        ST   M3
        LD   M5
        ST   M4
        LD   M6
        ST   M5
        LD   M0             ; append the new digit
        ST   M6
        JMP  main

clear:  LDI  A, 0
        LDI  B, 14          ; M1 to ME
clr:    ST   [B]
        DJNZ clr
        RET

add:    LDI  B, 6
        CLC
addl:   LD   [B+8]          ; total digit
        ADC  [B]            ; plus entry digit
        DA
        ST   [B+8]
        DJNZ addl
        RET

show:   LDI  B, 6           ; copy the total to the display
showl:  LD   [B+8]
        ST   [B]
        DJNZ showl
        RET
