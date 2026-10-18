# CalCo ISA, Revision A

CalCo is an 8-bit calculator coprocessor. Revision A drives a 6-digit adding
machine. It works with decimal digits, one digit per byte.

## Programmer model

| Item  | Size      | Notes                                       |
| ----- | --------- | ------------------------------------------- |
| A     | 8 bit     | Accumulator. All ALU results go here.       |
| B     | 8 bit     | Pointer for `[B+n]` and counter for `DJNZ`. |
| PC    | 8 bit     | Wraps from 0xFF to 0x00.                    |
| Z     | 1 bit     | Zero flag.                                  |
| C     | 1 bit     | Carry flag. Set on borrow for subtraction.  |
| Stack | 2 x 8 bit | Return addresses for `CALL` and `RET`.      |

Memory:

- ROM: 256 bytes of program, 0x00 to 0xFF. Code cannot read ROM as data.
- RAM: 16 bytes, named `M0` to `MF`. The board decides which cells it shows.

The stack is a ring of two entries. A third `CALL` overwrites the oldest return
address.

Reset state: PC = 0x00, A = 0, B = 0, Z = C = 0. RAM and stack contents are
undefined.

Every instruction takes one cycle per byte.

## Encoding

The high nibble is the opcode. The low nibble is the operand.

```
oooo nnnn             one byte
oooo nnnn aaaaaaaa    two bytes, second byte is an address or immediate
```

For memory instructions, bit 7 selects the addressing mode:

| Bit 7 | Mode    | Address          | Example           |
| ----- | ------- | ---------------- | ----------------- |
| 0     | Direct  | n                | `14` = `LD M4`    |
| 1     | Indexed | (B + n) and 0x0F | `94` = `LD [B+4]` |

Indexed addresses wrap inside RAM. `[B+3]` with B = 0x0E reads `M1`.

## Instructions

`*` means the flag is set from the result. `-` means unchanged.

| Hex   | Mnemonic    | Operation                           | Z | C |
| ----- | ----------- | ----------------------------------- | - | - |
| 00    | `NOP`       | Nothing                             | - | - |
| 01    | `HLT`       | Wait for a key press, then continue | - | - |
| 02    | `RET`       | PC <- pop                           | - | - |
| 03    | `SWP`       | Swap A and B                        | - | - |
| 04    | `CLC`       | C <- 0                              | - | 0 |
| 05    | `SEC`       | C <- 1                              | - | 1 |
| 06    | `INC`       | A <- A + 1                          | * | - |
| 07    | `DEC`       | A <- A - 1                          | * | - |
| 08    | `DA`        | Decimal adjust after `ADC`          | * | * |
| 09    | `DS`        | Decimal adjust after `SBC`          | * | - |
| 1n    | `LD Mn`     | A <- Mn                             | - | - |
| 2n    | `ST Mn`     | Mn <- A                             | - | - |
| 30 ii | `LDI A, ii` | A <- ii                             | - | - |
| 31 ii | `LDI B, ii` | B <- ii                             | - | - |
| 32 ii | `CPI ii`    | Compute A - ii, discard result      | * | * |
| 4n    | `ADC Mn`    | A <- A + Mn + C                     | * | * |
| 5n    | `SBC Mn`    | A <- A - Mn - C                     | * | * |
| 6n    | `CMP Mn`    | Compute A - Mn, discard result      | * | * |
| 7p    | `IN p`      | A <- port p                         | - | - |
| 8p    | `OUT p`     | port p <- A                         | - | - |
| 9n    | `LD [B+n]`  | A <- M(B+n)                         | - | - |
| An    | `ST [B+n]`  | M(B+n) <- A                         | - | - |
| Bc aa | `Jcc aa`    | If condition c holds, PC <- aa      | - | - |
| Cn    | `ADC [B+n]` | A <- A + M(B+n) + C                 | * | * |
| Dn    | `SBC [B+n]` | A <- A - M(B+n) - C                 | * | * |
| En    | `CMP [B+n]` | Compute A - M(B+n), discard result  | * | * |
| F0 aa | `CALL aa`   | push PC + 2, PC <- aa               | - | - |
| F1 aa | `DJNZ aa`   | B <- B - 1, if B != 0 then PC <- aa | - | - |

All encodings not listed are reserved. Their behavior is undefined.

Notes:

- There is no plain `ADD` or `SUB`. Use `CLC` first, then `ADC` or `SBC`.
- `INC` and `DEC` do not change C.
- `DJNZ` with B = 0 loops 256 times.
- `[B]` is short for `[B+0]`.

### Conditions

| c | Mnemonic | Jumps if |
| - | -------- | -------- |
| 0 | `JMP`    | always   |
| 1 | `JZ`     | Z = 1    |
| 2 | `JNZ`    | Z = 0    |
| 3 | `JC`     | C = 1    |
| 4 | `JNC`    | C = 0    |

After `CMP` or `CPI`, `JC` jumps if A is lower and `JZ` jumps if A is equal.

### Decimal adjust

Digits are stored as plain values 0 to 9. `DA` and `DS` keep them in that range
and carry to the next digit.

```
DA:  if A >= 10:  A <- A - 10, C <- 1
     else:        C <- 0

DS:  if C = 1:    A <- A + 10
```

`DA` expects A in the range 0 to 19. `DS` keeps C set so the borrow moves to the
next digit.

## Ports

| Port | Name  | Dir | Function                                            |
| ---- | ----- | --- | --------------------------------------------------- |
| 0    | `KEY` | in  | Last key code, or 0xFF if none. Reading clears it.  |
| F    | `REV` | in  | Chip revision. Rev A returns 0x10.                  |
| 0    | `ANN` | out | Annunciators. The board decides what each bit does. |

The board defines the key codes and the annunciators. See the device docs,
for example [Summa 6](../devices/summa6.md).

## Errata

| Bug                                | Affects                       |
| ---------------------------------- | ----------------------------- |
| `DS` clears C when the result is 0 | Rev A, date codes before 7624 |

When `DS` adds 10 and the result is 0, C is cleared instead of kept. The borrow
to the next digit is lost.

## Assembly syntax

```
label:  MNEMONIC operand   ; comment
```

- Numbers are decimal or hex with `0x`.
- RAM cells are `M0` to `MF`. Ports use the names above or a number.
- `.org addr` sets the address of the next instruction.

## Example

Add the Summa 6 display to a total stored in `M9` to `ME`. Both numbers have the most
significant digit first, so the loop runs from the rightmost digit to the left.

```
ADDR  BYTES   SOURCE
                      .org 0x40
0040  31 06   add:    LDI  B, 6
0042  04              CLC
0043  98      loop:   LD   [B+8]     ; total digit
0044  C0              ADC  [B]       ; plus display digit
0045  08              DA
0046  A8              ST   [B+8]
0047  F1 43           DJNZ loop
0049  02              RET
```
