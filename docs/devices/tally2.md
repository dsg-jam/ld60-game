# Tally 2

The Tally 2 is a 2-digit tally counter by DSG Electronics. It runs a
[CalCo](../isa/calco.md) chip with the firmware in a 256 byte EPROM socket.
It is the first job on the bench.

Early units have a CalCo Rev A, which only supports a few instructions. Later
units have a Rev B. Check the chip sticker before you change the firmware.

## Display

`M1` and `M2` are shown on the display, left to right. `M2` is the rightmost
digit. Values above 9 show broken segment patterns.

## Key codes

| Code | Key |
| ---- | --- |
| 0A   | +   |
| 0D   | C   |

The codes match the Summa 6, so firmware habits carry over.

## Power on

RAM holds random values after power on. The firmware has to clear the display.
