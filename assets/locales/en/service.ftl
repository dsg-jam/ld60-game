service-footer = DSG ELECTRONICS   { $device }

service-summary-tally2 = 2-digit tally counter. Service sheet for board { $board }.
service-summary-summa6 = 6-digit adding machine. Service sheet for board { $board }.
service-summary-lock4 = Door lock with a 4-digit code. Service sheet for board { $board }.

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
service-annunciator-open = ANN bit { $bit } lights OPEN.
service-annunciator-shut = ANN bit { $bit } lights SHUT.

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

service-use = HOW TO USE
service-use-lock4-open = Type the code and press #. The right code lights OPEN. The code is 0000 after power on.
service-use-lock4-entry = The display keeps the digits after #. The next digit starts a new entry. Storing a new code clears the display. So does * on an open lock.
service-use-lock4-wrong = A wrong code keeps the lock shut. After 3 wrong codes in a row, the lock lights the error mark and ignores all keys until power off. A right code resets the count.
service-use-lock4-clear = C clears the entry and shuts the lock. It does not reset the count.
service-use-lock4-change = To change the code, open the lock and press *. OPEN and SHUT light together. Type the new code and press #. The lock shuts.
