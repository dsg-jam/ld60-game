# Lock 4

The Lock 4 is a door lock with a 4-digit code by DSG Electronics. It runs a
[CalCo](../isa/calco.md) chip with the firmware in a 256 byte EPROM socket.
The chip is always a CalCo Rev C.

## Use

Type the code, then press `#`. The right code opens the lock and lights OPEN. A
wrong code keeps it shut. After 3 wrong codes in a row the lock stops listening
and lights the error mark until it is powered off. A right code resets the
count. `C` clears the entry and shuts the lock, but it does not reset the count.

The code is `0000` after power on. To change it, open the lock and press `*`.
OPEN and SHUT both light up. Type the new code and press `#`. The lock shuts
with the new code.

The display shows the last 4 digits typed. After `#` it keeps them until the
next digit starts a new entry. Storing a new code clears the display. So does
`*` on an open lock.

## Board revisions

The board around the chip comes in three revisions. They wire the display, the
keys and the annunciators differently. The sticker under the device shows the
revision, for example `BOARD 4B`. The serial number starts with it too. The
service sheet of a job lists the I/O of its board.

| Board | Display   | `#` | `*` | `C` | Open  | Shut  | Error |
| ----- | --------- | --- | --- | --- | ----- | ----- | ----- |
| 4A    | `M1`-`M4` | 0A  | 0B  | 0C  | bit 0 | bit 1 | bit 2 |
| 4B    | `M6`-`M9` | 0E  | 0D  | 0F  | bit 2 | bit 0 | bit 1 |
| 4C    | `MB`-`ME` | 0B  | 0C  | 0A  | bit 1 | bit 3 | bit 0 |

## Display

The display shows four RAM cells, left to right. On board 4A these are `M1` to
`M4`, and `M4` is the rightmost digit. Values above 9 show broken segment
patterns.

## Key codes

The digit keys 0 to 9 send the codes 00 to 09 on every board. The codes of `#`,
`*` and `C` depend on the board.

## Annunciators

Each annunciator is lit by one `ANN` bit. The bit depends on the board. The
display text names the lit lamps after the digits, like `0000 OPEN`. The error
mark shows as `E`.

## Power on

RAM holds random values after power on. The firmware has to clear the display
and set the code.

## End of line tests

The factory runs these tests on every unit. Power on the device, then press the
keys of each line and check the display. Spaces only group the keys.

| Keys          | Display        |
| ------------- | -------------- |
| (power on)    | 0000 SHUT      |
| `C 1234`      | 1234 SHUT      |
| `C #`         | 0000 OPEN      |
| `C 12#`       | 0012 SHUT      |
| `C #C`        | 0000 SHUT      |
| `C 1#2#3#`    | E              |
| `C #*`        | 0000 OPEN SHUT |
| `C #*42# 42#` | 0042 OPEN      |

The customer can test more than this list.

## Known faults

- Liquid damage can leave a RAM bit stuck at 0 or 1.
- Old EPROMs can lose bits.
