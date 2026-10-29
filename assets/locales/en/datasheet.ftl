datasheet-footer = DSG ELECTRONICS  CALCO REV { $revision }
datasheet-title = CALCO REV { $revision }
datasheet-subtitle = The chip runs the firmware one line at a time, from the top down.

datasheet-has = WHAT IT HAS
datasheet-register-a = Holds the number the chip works on.
datasheet-register-b = A second number. It counts loops and picks cells like [B+2].
datasheet-cells = 16 cells. Each holds a number from 0 to 255. The gadget manual says which cells are the display.
datasheet-register-stack = Remembers where to come back to after CALL.
datasheet-reset-b = At power on, A and B are 0 and the cells hold random numbers.
datasheet-reset = At power on, A is 0 and the cells hold random numbers.

datasheet-group-numbers = NUMBERS
datasheet-group-keys = KEYS
datasheet-group-jumps = JUMPS
datasheet-group-math = MATH
datasheet-group-routines = ROUTINES
datasheet-group-loops = LOOPS
datasheet-group-lights = LIGHTS

datasheet-experts = FOR EXPERTS
datasheet-flags = A compare sets Z if A was equal and C if A was lower. The Z and C columns show which other lines change them.
datasheet-ports = PORTS
datasheet-syntax = SYNTAX
datasheet-operation = OPERATION
datasheet-flag-legend = * from result   - unchanged   0 cleared   1 set
datasheet-operand-legend = n: cell or offset   ii: byte   aa: address   p: port

datasheet-errata = ERRATA
datasheet-no-errata = No known bugs in Rev { $revision }.
datasheet-affects = Affects: { $chips }
