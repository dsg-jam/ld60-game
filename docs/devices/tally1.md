# Tally 1

The Tally 1 is a 1-digit tally counter by DSG Electronics. It runs a
[CalCo](../isa/calco.md) chip with the firmware in a 256 byte EPROM socket. It
has a single `+` key. The first jobs on the bench are Tally 1 units.

## Board revisions

The board around the chip comes in three revisions. They wire the display and
the key differently. The sticker under the device shows the revision, for
example `BOARD 1B`. The serial number starts with it too.

| Board | Display | `+` |
| ----- | ------- | --- |
| 1A    | `M2`    | 0A  |
| 1B    | `M5`    | 0B  |
| 1C    | `M1`    | 01  |

## Display

The display shows one RAM cell. Values above 9 show broken segment patterns.

The Tally 1 has no annunciators.

## Power on

RAM holds random values after power on. The firmware has to clear the display.

## End of line tests

The factory runs these tests on every unit. Power on the device, then press the
keys of each line and check the display. Spaces only group the keys.

| Keys          | Display |
| ------------- | ------- |
| (power on)    | 0       |
| `+`           | 1       |
| `+++`         | 3       |
| `+++++ +++++` | 0       |

After 9, the count starts again at 0. The customer can test more than this list.
