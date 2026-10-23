datasheet-footer = DSG ELECTRONICS  CALCO REV { $revision }
datasheet-title = CALCO
datasheet-subtitle = 8-bit calculator coprocessor. One decimal digit per byte.

datasheet-registers = REGISTERS
datasheet-stack = Stack
datasheet-register-a = Accumulator. All ALU results go here.
datasheet-register-b = Pointer for [B+n], counter for DJNZ.
datasheet-register-pc = Program counter. Wraps from FF to 00.
datasheet-register-flags = Zero and carry flags. C is set on borrow.
datasheet-register-stack = Two return addresses for CALL and RET.

datasheet-memory = MEMORY
datasheet-rom = ROM: 256 bytes, 00 to FF. Code cannot read it as data.
datasheet-ram = RAM: 16 cells, M0 to MF.
datasheet-reset-b-stack = Reset: PC, A, B, Z and C are 0. RAM and stack are undefined.
datasheet-reset-b = Reset: PC, A, B, Z and C are 0. RAM is undefined.
datasheet-reset-stack = Reset: PC, A, Z and C are 0. RAM and stack are undefined.
datasheet-reset = Reset: PC, A, Z and C are 0. RAM is undefined.

datasheet-ports = PORTS

datasheet-instructions = INSTRUCTION SET
datasheet-syntax = SYNTAX
datasheet-operation = OPERATION
datasheet-flag-legend = * from result   - unchanged   0 cleared   1 set
datasheet-operand-legend = n: cell or offset   ii: byte   aa: address   p: port

datasheet-conditions = CONDITIONS
datasheet-condition-always = always jumps
datasheet-condition-zero = jumps if Z = 1
datasheet-condition-not-zero = jumps if Z = 0
datasheet-condition-carry = jumps if C = 1
datasheet-condition-no-carry = jumps if C = 0
datasheet-compare-cmp = After CMP or CPI, JC jumps if A is lower and JZ if A is equal.
datasheet-compare-cpi = After CPI, JC jumps if A is lower and JZ if A is equal.
datasheet-notes = NOTES

datasheet-errata = ERRATA
datasheet-no-errata = No known bugs in Rev { $revision }.
datasheet-affects = Affects: { $chips }
