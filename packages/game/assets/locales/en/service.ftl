service-footer = DSG ELECTRONICS   { $device }

service-summary-tally2 = 2-digit tally counter. Service sheet for board { $board }.
service-summary-summa6 = 6-digit adding machine. Service sheet for board { $board }.

service-key-codes = KEY CODES
service-range = { $first } to { $last }

service-display = DISPLAY
service-display-cells =
    { $digits ->
        [one] { $digits } digit
       *[other] { $digits } digits
    }, { $first } to { $last } from left to right.
service-display-broken = Values above 9 show broken segments.

service-annunciators = ANNUNCIATORS
service-annunciator-minus = ANN bit { $bit } lights the minus sign.
service-annunciator-error = ANN bit { $bit } lights the error mark.

service-board = BOARD { $board }
service-board-chip = CalCo chip, firmware in a 256 byte EPROM socket.
service-board-ram = RAM holds random values after power on.

service-known-faults = KNOWN FAULTS
service-fault-liquid = Liquid can leave a RAM bit stuck at 0 or 1.
service-fault-eprom = Old EPROMs can lose bits.

service-line-tests = END OF LINE TESTS
service-line-tests-factory = Every unit passes these at the factory.
service-line-tests-how = Power on, then press the keys of each line and check the display.
service-line-tests-spaces = Spaces only group the keys.
service-line-tests-keys = KEYS
service-line-tests-display = DISPLAY
service-line-tests-power-on = (power on)
service-line-tests-flash = Flashing the firmware also powers the unit on.
