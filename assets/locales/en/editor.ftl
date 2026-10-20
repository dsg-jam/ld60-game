editor-title-source = FIRMWARE EDITOR
editor-title-notes = NOTES
editor-title-versions = FIRMWARE VERSIONS

editor-command-notes = F2 NOTES
editor-command-versions = F3 VERSIONS
editor-command-assemble = F5 ASSEMBLE
editor-command-flash = F6 FLASH

editor-no-job = NO JOB ON THE BENCH
editor-no-more-jobs = NO MORE JOBS TONIGHT.
editor-no-device = NO DEVICE CONNECTED
editor-assembled =
    { $bytes ->
        [one] ASSEMBLED, { $bytes } BYTE
       *[other] ASSEMBLED, { $bytes } BYTES
    }
editor-flashed = FLASHED. DEVICE RESTARTED.

editor-assembly-failed = ASSEMBLY FAILED
editor-asm-error = line { $line }: { $message }
editor-asm-error-more = line { $line }: { $message } (+{ $more } MORE)
editor-asm-invalid-label = invalid label '{ $name }'
editor-asm-invalid-number = invalid number '{ $text }'
editor-asm-invalid-operand = invalid operand '{ $text }'
editor-asm-directive-expects-numbers = .{ $directive } expects numbers
editor-asm-org-expects-address = .org expects one address
editor-asm-db-expects-value = .db expects at least one value
editor-asm-unknown-directive = unknown directive '.{ $directive }'
editor-asm-duplicate-label = duplicate label '{ $name }'
editor-asm-outside-rom = address { $address } is outside the ROM
editor-asm-rom-full = code does not fit in the ROM
editor-asm-overlap = overlaps code at { $address }
editor-asm-word-range = value { $value } does not fit in a word
editor-asm-unknown-label = unknown label '{ $name }'
editor-asm-expects-value = expected a number or label
editor-asm-unknown-mnemonic = unknown mnemonic '{ $mnemonic }'
editor-asm-unsupported = '{ $form }' is not supported by { $chip }
editor-asm-takes-no-operands = { $mnemonic } takes no operands
editor-asm-expects-one-operand = { $mnemonic } expects one operand
editor-asm-expects-register-and-value = { $mnemonic } expects a register and a value
editor-asm-expects-a-or-b = { $mnemonic } expects register A or B
editor-asm-expects-memory = { $mnemonic } expects Mn or [B+n]
editor-asm-offset-range = offset must be 0 to 15
editor-asm-unknown-port = unknown port '{ $name }'
editor-asm-port-range = port must be 0 to 15
editor-asm-expects-port = expected a port
editor-asm-byte-range = value { $value } is outside 0 to 255
editor-error-parse = invalid level file at line { $line }, column { $column }
editor-error-write = cannot write the level file
editor-error-board-not-allowed = the job does not allow board revision { $board }
editor-error-no-rewrite = no rewrite for board revision { $board } passes the checks
editor-error-chip-mismatch = { $device } cannot use a { $chip } chip
editor-error-rom-size =
    { $actual ->
        [one] ROM has { $actual } word, expected { $expected }
       *[other] ROM has { $actual } words, expected { $expected }
    }
editor-burn-mask-rom = BURN FAILED: a mask ROM cannot be programmed
editor-burn-size-mismatch = BURN FAILED: the image does not match the ROM size
editor-burn-bit-conflict = BURN FAILED: { $address } needs a bit set from 0 to 1. Erase the chip first.

editor-action-save = S SAVE
editor-action-restore = ENTER RESTORE
editor-action-remove = DEL REMOVE

editor-saved = SAVED AS VERSION { $version }
editor-duplicate = SAME AS VERSION { $version }
editor-full = NO ROOM. REMOVE A VERSION FIRST.
editor-restored = RESTORED VERSION { $version }
editor-restored-kept = RESTORED VERSION { $version }. KEPT YOUR CHANGES AS { $kept }.
editor-full-keep = NO ROOM TO KEEP YOUR CHANGES. REMOVE A VERSION FIRST.
editor-factory-stays = THE FACTORY VERSION STAYS
editor-remove-armed = PRESS DEL AGAIN TO REMOVE VERSION { $version }
editor-removed = REMOVED VERSION { $version }

# The time on the job, the lines added and removed, and the assembler result.
editor-version-label = { $time }  { $changes }  { $result }
editor-version-factory = FACTORY
editor-version-bytes =
    { $bytes ->
        [one] { $bytes } BYTE
       *[other] { $bytes } BYTES
    }
editor-version-errors =
    { $errors ->
        [one] { $errors } ERROR
       *[other] { $errors } ERRORS
    }
editor-version-error = ERROR
editor-version-auto = AUTO
editor-version-flashed = FLASHED

editor-command-test = F4 TEST
editor-command-trace-on = F7 TRACE ON
editor-command-trace-off = F7 TRACE OFF
editor-command-break = F9 BREAK
editor-command-step = F10 STEP
editor-command-go = F11 GO

editor-bench-pass = TEST PASSED. THE CUSTOMER WILL FIND NOTHING WRONG.
editor-bench-dead = TEST FAILED: THE DEVICE DOES NOT START
editor-bench-wrong-start = TEST FAILED: AT START IT SHOWS { $shown }, NOT { $expected }
editor-bench-wrong = TEST FAILED: AFTER { $keys } IT SHOWS { $shown }, NOT { $expected }
editor-bench-froze-start = TEST FAILED: IT FREEZES AT START
editor-bench-froze = TEST FAILED: IT FREEZES AFTER { $keys }

editor-debug-line = LINE { $line }
editor-debug-address = ADDRESS { $address }
editor-debug-held = CHIP HELD AT { $place }
editor-debug-stepped = STEPPED TO { $place }
editor-debug-waiting = THE CHIP WAITS FOR A KEY AT { $place }. PRESS ONE ON THE DEVICE.
editor-debug-break = STOPPED AT THE BREAKPOINT ON { $place }
editor-debug-running = CHIP RUNNING. IT WAITS FOR A KEY.
editor-debug-timeout = CHIP RUNNING. IT NEVER WAITS FOR A KEY.
editor-debug-not-held = THE CHIP IS ALREADY RUNNING
editor-debug-ram = RAM  { $cells }

editor-break-set = BREAKPOINT ON LINE { $line }
editor-break-cleared = BREAKPOINT REMOVED FROM LINE { $line }
editor-break-no-code = LINE { $line } HAS NO CODE IN THE CHIP
