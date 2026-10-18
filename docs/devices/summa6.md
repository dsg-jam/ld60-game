# Summa 6

The Summa 6 is a 6-digit adding machine by DSG Electronics. It runs a
[CalCo](../isa/calco.md) chip with the firmware in a 256 byte EPROM socket.

## Board revisions

The board around the chip comes in three revisions. They wire the display,
the keys and the annunciators differently. The sticker under the device shows
the revision, for example `BOARD 6B`. The serial number starts with it too.
The service sheet of a job lists the I/O of its board.

| Board | Display    | `+` | `-` | `=` | `C` | Minus | Error |
| ----- | ---------- | --- | --- | --- | --- | ----- | ----- |
| 6A    | `M1`-`M6`  | 0A  | 0B  | 0C  | 0D  | bit 0 | bit 1 |
| 6B    | `M2`-`M7`  | 0C  | 0D  | 0E  | 0F  | bit 1 | bit 0 |
| 6C    | `M8`-`MD`  | 0B  | 0A  | 0D  | 0C  | bit 2 | bit 3 |

## Display

The display shows six RAM cells, left to right. On board 6A these are `M1` to
`M6`, and `M6` is the rightmost digit. Values above 9 show broken segment
patterns.

## Key codes

The digit keys 0 to 9 send the codes 00 to 09 on every board. The codes of
`+`, `-`, `=` and `C` depend on the board.

## Annunciators

Each annunciator is lit by one `ANN` bit. The bit depends on the board.

## Power on

RAM holds random values after power on. The firmware has to clear the display.

## Known faults

- Liquid damage can leave a RAM bit stuck at 0 or 1.
- Old EPROMs can lose bits.
