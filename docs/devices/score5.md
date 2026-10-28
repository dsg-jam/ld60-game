# Score 5

The Score 5 is a scoreboard by DSG Electronics. It keeps the score of two sides
and the period of the game. It runs a [CalCo](../isa/calco.md) chip with the
firmware in a 256 byte EPROM socket. The chip is a CalCo Rev B or Rev C.

## Use

The display shows the home score, the period and the guest score, like `03204`:
home 03, period 2, guest 04.

`H` adds points to the home score and `G` adds them to the guest score. A side
gets 1 point, unless a digit key was pressed before. Then it gets that many
points, so `3H` adds 3. After each score the next one is 1 point again. A score
stops at 99.

`P` moves to the next period. The period stops at 4. `C` clears both scores and
goes back to period 1.

## Board revisions

The board around the chip comes in three revisions. They wire the display and
the keys differently. The sticker under the device shows the revision, for
example `BOARD 5B`. The serial number starts with it too. The service sheet of a
job lists the I/O of its board.

| Board | Display   | `H` | `G` | `P` | `C` |
| ----- | --------- | --- | --- | --- | --- |
| 5A    | `M1`-`M5` | 0A  | 0B  | 0C  | 0D  |
| 5B    | `M9`-`MD` | 0D  | 0F  | 0E  | 0C  |
| 5C    | `M4`-`M8` | 0C  | 0A  | 0F  | 0B  |

## Display

The display shows five RAM cells, left to right. On board 5A these are `M1` to
`M5`. `M1` and `M2` are the home score, `M3` is the period, and `M4` and `M5`
are the guest score. Values above 9 show broken segment patterns.

## Key codes

The digit keys 0 to 9 send the codes 00 to 09 on every board. The codes of `H`,
`G`, `P` and `C` depend on the board.

## Annunciators

The Score 5 has none.

## Power on

RAM holds random values after power on. The firmware has to clear the scores and
set period 1.

## End of line tests

The factory runs these tests on every unit. Power on the device, then press the
keys of each line and check the display. Spaces only group the keys.

| Keys                                    | Display |
| --------------------------------------- | ------- |
| (power on)                              | 00100   |
| `C H`                                   | 01100   |
| `C GG`                                  | 00102   |
| `C 3H 2G`                               | 03102   |
| `C 2HH`                                 | 03100   |
| `C 5G 5G`                               | 00110   |
| `C 9H 9H 9H 9H 9H 9H 9H 9H 9H 9H 9H 9H` | 99100   |
| `C PP`                                  | 00300   |
| `C PPPPP`                               | 00400   |

The customer can test more than this list.

## Known faults

- Liquid damage can leave a RAM bit stuck at 0 or 1.
- Old EPROMs can lose bits.
