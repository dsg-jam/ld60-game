; Summa-6 firmware 1.0
;
; M1-M6  display and entry, M6 is the rightmost digit
; M7     pending operation: 0 plus, 1 minus
; M8     1 if the next digit starts a new number
; M9-ME  total
; MF     scratch

reset:  JMP  main
main:   HLT
        IN   KEY
        CPI  10
        JC   digit
        CPI  0x0A
        JZ   plus
        CPI  0x0B
        JZ   minus
        CPI  0x0D
        JZ   clrkey
equals: CALL apply
        LDI  A, 0
        LDI  B, 6
eqclr:  ST   [B+8]
        DJNZ eqclr
        ST   M7
        JMP  done

plus:   CALL apply
        LDI  A, 0
        ST   M7
        JMP  done

minus:  CALL apply
        LDI  A, 1
        ST   M7
done:   LDI  A, 1
        ST   M8
        JMP  main

clrkey: CALL clear
        JMP  main

digit:  ST   MF
        LD   M8
        CPI  0
        JZ   shift
        LDI  A, 0
        LDI  B, 6
dclr:   ST   [B]
        DJNZ dclr
        ST   M8
shift:  LD   M2
        ST   M1
        LD   M3
        ST   M2
        LD   M4
        ST   M3
        LD   M5
        ST   M4
        LD   M6
        ST   M5
        LD   MF
        ST   M6
        JMP  main

apply:  LD   M7
        DEC
        JZ   asub
        CALL add
        JMP  ashow
asub:   CALL sub
ashow:  LDI  B, 6
ashl:   LD   [B+8]
        ST   [B]
        DJNZ ashl
        RET

add:    LDI  B, 6
        CLC
aloop:  LD   [B+8]
        ADC  [B]
        DA
        ST   [B+8]
        DJNZ aloop
        RET

sub:    LDI  B, 6
        CLC
sloop:  LD   [B+8]
        SBC  [B]
        DS
        ST   [B+8]
        DJNZ sloop
        RET

clear:  LDI  A, 0
        LDI  B, 6
cloop:  ST   [B]
        ST   [B+8]
        DJNZ cloop
        ST   M7
        ST   M8
        RET
