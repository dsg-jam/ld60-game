# Summa 6

The Summa 6 is a 6-digit adding machine by DSG Electronics. It runs a
[CalCo](../isa/calco.md) chip with the firmware in a 256 byte EPROM socket.

## Display

`M1` to `M6` are shown on the display, left to right. `M6` is the rightmost
digit. Values above 9 show broken segment patterns.

## Key codes

| Code  | Key    |
| ----- | ------ |
| 00-09 | 0 to 9 |
| 0A    | +      |
| 0B    | -      |
| 0C    | =      |
| 0D    | C      |

## Annunciators

| `ANN` bit | Shows      |
| --------- | ---------- |
| 0         | Minus sign |
| 1         | Error mark |

## Power on

RAM holds random values after power on. The firmware has to clear the display.

## Known faults

- Liquid damage can leave a RAM bit stuck at 0 or 1.
- Old EPROMs can lose bits.
