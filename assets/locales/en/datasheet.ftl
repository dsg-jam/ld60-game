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

# The rows of the instruction table, named after their hex code.
datasheet-op-00 =
    .operation = Nothing
    .meaning = Do nothing.
datasheet-op-01 =
    .operation = Wait for a key press
    .meaning = Wait for the next key press.
datasheet-op-02 =
    .operation = PC <- pop
    .meaning = Go back to the line after the CALL.
datasheet-op-03 =
    .operation = Swap A and B
    .meaning = Swap the numbers in A and B.
datasheet-op-04 =
    .operation = C <- 0
    .meaning = Clear the carry. Do this before ADC or SBC.
datasheet-op-05 =
    .operation = C <- 1
    .meaning = Set the carry.
datasheet-op-06 =
    .operation = A <- A + 1
    .meaning = Add 1 to A.
datasheet-op-07 =
    .operation = A <- A - 1
    .meaning = Take 1 from A.
datasheet-op-08 =
    .operation = Decimal adjust after ADC
    .meaning = After ADC: if A is 10 or more, take 10 off and carry 1.
datasheet-op-09 =
    .operation = Decimal adjust after SBC
    .meaning = After SBC: if 1 was borrowed, add 10 back to A.
    .note = DS keeps the carry set.
datasheet-op-1n =
    .operation = A <- Mn
    .meaning = Copy cell M2 to A.
datasheet-op-2n =
    .operation = Mn <- A
    .meaning = Copy A to cell M2.
datasheet-op-30-ii =
    .operation = A <- ii
    .meaning = Put 5 in A.
datasheet-op-31-ii =
    .operation = B <- ii
    .meaning = Put 4 in B.
datasheet-op-32-ii =
    .operation = A - ii, result dropped
    .meaning = Compare A with 10. A does not change.
datasheet-op-4n =
    .operation = A <- A + Mn + C
    .meaning = Add cell M2 and the carry to A.
    .note = There is no plain add. Use CLC first, then ADC.
datasheet-op-5n =
    .operation = A <- A - Mn - C
    .meaning = Take cell M2 and the carry from A.
    .note = There is no plain subtract. Use CLC first, then SBC.
datasheet-op-6n =
    .operation = A - Mn, result dropped
    .meaning = Compare A with cell M2. A does not change.
datasheet-op-7p =
    .operation = A <- port p
    .meaning = Put the code of the pressed key in A. The gadget manual lists the codes.
datasheet-op-8p =
    .operation = port p <- A
    .meaning = Light the marks of the display. Each bit of A is one mark.
datasheet-op-9n =
    .operation = A <- M(B+n)
    .meaning = Copy the cell 2 after B to A. With B at 3, that is M5.
    .note = Cells wrap around: with B at 14, [B+3] is M1.
datasheet-op-an =
    .operation = M(B+n) <- A
    .meaning = Copy A to the cell 2 after B.
datasheet-op-b0-aa =
    .operation = PC <- aa
    .meaning = Go to the line labeled count.
datasheet-op-b1-aa =
    .operation = If Z = 1, PC <- aa
    .meaning = After a compare: go to done if A was equal.
datasheet-op-b2-aa =
    .operation = If Z = 0, PC <- aa
    .meaning = After a compare: go to done if A was not equal.
datasheet-op-b3-aa =
    .operation = If C = 1, PC <- aa
    .meaning = After a compare: go to done if A was lower.
datasheet-op-b4-aa =
    .operation = If C = 0, PC <- aa
    .meaning = After a compare: go to done if A was not lower.
datasheet-op-cn =
    .operation = A <- A + M(B+n) + C
    .meaning = Add the cell 2 after B and the carry to A.
datasheet-op-dn =
    .operation = A <- A - M(B+n) - C
    .meaning = Take the cell 2 after B and the carry from A.
datasheet-op-en =
    .operation = A - M(B+n), result dropped
    .meaning = Compare A with the cell 2 after B.
datasheet-op-f0-aa =
    .operation = push PC + 2, PC <- aa
    .meaning = Run the lines from show until RET, then come back.
    .note = Only two CALLs can wait to come back. A third one forgets the oldest.
datasheet-op-f1-aa =
    .operation = B <- B - 1, jump if B != 0
    .meaning = Take 1 from B, then go to next unless B is 0.
    .note = With B at 0, DJNZ goes round 256 times.

# The ports, named after the port.
datasheet-port-key = Code of the last key, 255 if none. Reading it takes it.
datasheet-port-rev = Chip revision: 16 on Rev A, 32 on Rev B, 48 on Rev C.
datasheet-port-ann = Marks of the display. The board decides what each bit shows.

# The known chip bugs, named after `Erratum::id`.
erratum-ds-clears-carry-on-zero =
    .title = DS CLEARS C WHEN THE RESULT IS 0
    .description = When DS adds 10 and the result is 0, C is cleared instead of kept. The borrow to the next digit is lost.
    .affects = Rev B, date codes before 7624
