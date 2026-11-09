# Tick

The 4-bit chip in watch movements, the first chip generation in the game.

- One instruction per 4-bit word. No operands, registers or jumps.
- The program counter wraps from the last word of ROM to 0.
- A crystal ticks once a second. `WAIT` sleeps until the next tick.
- An erased EPROM word is `1111`, which does nothing. Programming can only clear
  bits, so changing a written word needs an erase.

| Word   | Name | Effect                         | Revisions |
| ------ | ---- | ------------------------------ | --------- |
| `1111` | NOP  | Nothing                        | A, B      |
| `0001` | STEP | Moves the hand one mark on     | A, B      |
| `0010` | WAIT | Sleeps until the next tick     | A, B      |
| `0100` | HOME | Moves the hand back to the top | B         |

Other words do nothing.

## Revisions

- A: 4 words of ROM.
- B: 8 words of ROM and a key. A key press sends the program to address 7 and
  wakes the chip.

## The watch movement

The hand points at one of 12 marks. Mark 0 is the top.
