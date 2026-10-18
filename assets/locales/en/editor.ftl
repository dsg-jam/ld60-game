editor-title-source = FIRMWARE EDITOR
editor-title-notes = NOTES
editor-title-versions = FIRMWARE VERSIONS

editor-command-notes = F2 NOTES
editor-command-versions = F3 VERSIONS
editor-command-assemble = F5 ASSEMBLE
editor-command-flash = F6 FLASH
editor-command-clicks-on = F8 CLICKS ON
editor-command-clicks-off = F8 CLICKS OFF

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
editor-error-parse = invalid level file: { $message }
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
