; Score-5 firmware 1.0
;
; M1-M2  display, home score, M2 is the ones digit
; M3     period, 1 to 4
; M4-M5  guest score, M5 is the ones digit
; M6     points for the next score

reset:  LDI  A, 0
        ST   M1
        ST   M2
        ST   M4
        ST   M5
        LDI  A, 1
        ST   M3
one:    LDI  A, 1
        ST   M6
main:   WAIT
        IN   KEY
        CPI  10
        JC   digit
        CPI  10
        JZ   homekey
        CPI  11
        JZ   guestkey
        CPI  12
        JZ   nextkey
        CPI  13
        JZ   reset
        JMP  main

digit:  ST   M6
        JMP  main

nextkey:
        LD   M3
        CPI  4
        JNC  main
        INC
        ST   M3
        JMP  main

homekey:
        CLC
        LD   M2
        ADC  M6
        DA
        ST   M2
        JNC  one
        LD   M1
        INC
        CPI  10
        JNC  homecap
        ST   M1
        JMP  one
homecap:
        LDI  A, 9
        ST   M1
        ST   M2
        JMP  one

guestkey:
        CLC
        LD   M5
        ADC  M6
        DA
        ST   M5
        JNC  one
        LD   M4
        INC
        CPI  10
        JNC  guestcap
        ST   M4
        JMP  one
guestcap:
        LDI  A, 9
        ST   M4
        ST   M5
        JMP  one
