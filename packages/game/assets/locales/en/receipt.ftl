receipt-title-paid = PICKUP
receipt-title-handed-back = HANDED BACK
receipt-title-left = NOT REPAIRED

receipt-paid = PAID
receipt-amount = ${ $amount }
receipt-note = NOTE FROM THE CUSTOMER
receipt-till = TILL
receipt-till-total = ${ $amount } taken in so far.

note = { $complaint } { $reaction }

note-dead = It does not even wake up.
note-woke-said = I woke it up and the little window said { $shown }. It should say { $expected }.
note-woke-squiggles = I woke it up and the little window showed angry squiggles. It should say { $expected }.
note-pressed-said = I pressed { $keys } and the little window said { $shown }. It should say { $expected }.
note-pressed-squiggles = I pressed { $keys } and the little window showed angry squiggles. It should say { $expected }.
note-woke-froze = I woke it up and it froze stiff.
note-pressed-froze = I pressed { $keys } and it froze stiff.

note-tries-left =
    { $tries ->
        [one] Grrr! No money for this. One more try, then I take it home.
       *[other] Grrr! No money for this. You get { $tries } more tries.
    }
note-gave-up =
    { $tries ->
        [one] { $tries } try is enough. I take it home broken. No money for you.
       *[other] { $tries } tries is enough. I take it home broken. No money for you.
    }
