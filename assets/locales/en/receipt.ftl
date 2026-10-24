receipt-title-paid = PICKUP
receipt-title-handed-back = HANDED BACK
receipt-title-left = NOT REPAIRED

receipt-paid = PAID
receipt-tip = TIP
receipt-amount = ${ $amount }

receipt-bench = BENCH LOG
receipt-measure = MEASURE
receipt-this-fix = THIS FIX
receipt-factory-fix = FACTORY FIX
receipt-words = WORDS
receipt-cycles = CYCLES
receipt-tries = TRIES
receipt-value = { $value }
receipt-words-met = { $value }  NEAT
receipt-cycles-met = { $value }  QUICK
receipt-unknown = -
receipt-tries-used = { $tries } of { $of }
receipt-note-heading = NOTE FROM THE CUSTOMER
receipt-till = TILL
receipt-till-total = ${ $amount } in the till.

receipt-note = { $complaint } { $reaction }
receipt-note-paid = { $thanks } { $tip }

receipt-thanks-first =
    { $pick ->
        [0] It wakes up and says the right things again. My whole family pressed the keys all evening.
        [1] It works! I told my neighbor a human fixed it. My neighbor did not believe me.
       *[other] The little window shows the right things again. I will keep it far away from the soup this time.
    }
receipt-thanks-late =
    { $pick ->
        [0] It took you long, but now it works. I pay less for all my walking.
        [1] Finally the box listens to me. My feet are tired from walking here so often.
       *[other] It works now. Next time, make it work the first time, human.
    }
receipt-tip-neat = You changed only { $words } of its little thoughts. The old human book says { $par }. Take ${ $tip } extra for being careful.
receipt-tip-quick = It thinks as fast as a new one. Take ${ $tip } extra.
receipt-tip-neat-quick = You changed only { $words } of its little thoughts, and it thinks as fast as a new one. Take ${ $tip } extra.

receipt-note-dead = It does not even wake up.
receipt-note-woke-said = I woke it up and the little window said { $shown }. It should say { $expected }.
receipt-note-woke-squiggles = I woke it up and the little window showed angry squiggles. It should say { $expected }.
receipt-note-pressed-said = I pressed { $keys } and the little window said { $shown }. It should say { $expected }.
receipt-note-pressed-squiggles = I pressed { $keys } and the little window showed angry squiggles. It should say { $expected }.
receipt-note-woke-froze = I woke it up and it froze stiff.
receipt-note-pressed-froze = I pressed { $keys } and it froze stiff.

receipt-note-tries-left =
    { $tries ->
        [one] Grrr! No money for this. One more try, then I take it home.
       *[other] Grrr! No money for this. You get { $tries } more tries.
    }
receipt-note-gave-up =
    { $tries ->
        [one] { $tries } try is enough. I take it home broken. No money for you.
       *[other] { $tries } tries is enough. I take it home broken. No money for you.
    }
