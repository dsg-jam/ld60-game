receipt-title-paid = PICKUP
receipt-title-handed-back = HANDED BACK
receipt-title-left = NOT REPAIRED

receipt-paid = PAID
receipt-amount = ${ $amount }
receipt-note-heading = NOTE FROM THE CUSTOMER
receipt-till = TILL
receipt-till-total = ${ $amount } in the till.

receipt-note = { $complaint } { $reaction }

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
