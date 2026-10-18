# Tally 2

The Tally 2 is a 2-digit tally counter by DSG Electronics. It runs a
[CalCo](../isa/calco.md) chip with the firmware in a 256 byte EPROM socket.
It is the first job on the bench.

Early units have a CalCo Rev A, which only supports a few instructions. Later
units have a Rev B. Check the chip sticker before you change the firmware.

## Board revisions

The board around the chip comes in three revisions. They wire the display and
the keys differently. The sticker under the device shows the revision, for
example `BOARD 2B`. The serial number starts with it too. The service sheet of
a job lists the I/O of its board.

| Board | Display   | `+` | `C` |
| ----- | --------- | --- | --- |
| 2A    | `M1`-`M2` | 0A  | 0D  |
| 2B    | `M4`-`M5` | 0B  | 0C  |
| 2C    | `M0`-`M1` | 01  | 02  |

The codes of board 2A match the Summa 6 board 6A, so firmware habits carry
over.

## Display

The display shows two RAM cells, left to right. On board 2A these are `M1` and
`M2`, and `M2` is the rightmost digit. Values above 9 show broken segment
patterns.

The Tally 2 has no annunciators.

## Power on

RAM holds random values after power on. The firmware has to clear the display.

## End of line tests

The factory runs these tests on every unit. Power on the device, then press
the keys of each line and check the display. Spaces only group the keys.

| Keys       | Display |
| ---------- | ------- |
| (power on) | 00      |
| `C +`      | 01      |
| `C +++`    | 03      |
| `C +++C`   | 00      |

The customer can test more than this list.
