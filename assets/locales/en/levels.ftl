# Every level has a message named after its ID. { $board } is the board of
# the job, like 6A.

tally2-01 =
    .customer = Drob, door guard
    .complaint = I press C before the doors open, but it keeps counting from last night.
    .remark-tingle = C button poked with the tingle stick. Tingles fine.
    .remark-first-made = One of the first ones made. Still has its first thinking, version 0.9.

tally2-02 =
    .customer = Ploof, seat finder
    .complaint = My counter said 40 guests tonight. We have 20 chairs and nobody had to stand.
    .remark-works-fine = Both buttons and the little window work fine.
    .remark-first-thinking = Still has its first thinking, version 1.0.

tally2-03 =
    .customer = Hubb, bridge toll
    .complaint = Busy day. After 99 carts the left number turned into an angry squiggle.
    .remark-line-tests = Passes every factory test on the checklist.
    .remark-first-thinking = Still has its first thinking, version 1.0.

summa6-01 =
    .customer = Zib, bookkeeper
    .complaint = When I wake it up, the little window shows angry squiggles. The C button chases them away.
    .remark-squiggles = Angry squiggles in the little window every time it wakes up.
    .remark-first-thinking = Still has its first thinking, version 1.0.

summa6-02 =
    .customer = Grelk, fruit stall
    .complaint = My sums always end in 0. My buyers smile. I do not.
    .remark-licked = All buttons licked clean.
    .remark-all-spots = All six number spots light up every stroke.

summa6-03 =
    .customer = Vrask, tax counter
    .complaint = When I press 9 it gives me the answer. Taxes are full of 9s.
    .remark-tingle = 9 button poked with the tingle stick. Tingles fine.
    .remark-digits-fine = Buttons 0 to 8 make numbers like they should.

summa6-04 =
    .customer = Plimm, night counter
    .complaint = I spilled hot bean juice on it. Now it only takes away. A new little black brain costs more than the whole box.
    .remark-bean-juice = Dried bean juice on the board, around the part that remembers numbers. Wiped off.
    .remark-no-new-brain = Customer will not pay for a new little black brain.

summa6-05 =
    .customer = Quoob, potion mixer
    .complaint = 100 minus 91 shows 109 on this one. My other Summa says 9. Same thinking inside, I checked.
    .remark-first-brain = Its little black brain is one of the first ever made.
    .remark-twin = Same thinking as unit { $board }-05530, which works.

summa6-06 =
    .customer = Yimbo, student
    .complaint = 12 minus 34 is 999978? My money pouch says otherwise. Where is the minus mark?
    .remark-minus-mark = The minus mark lights up when every stroke is tested.
    .remark-less-than-nothing = Customer wants to see when there is less than nothing.

summa6-07 =
    .customer = Brakka, shop keeper
    .complaint = A trader gave it new thinking, version 1.1. It is shorter, he said. Now it freezes when I press +, - or =.
    .remark-trader = Thinking 1.1, put in by the trader's office.
    .remark-stuck = Gets stuck after +, - or =. Only putting it to sleep and waking it up again helps.

lock4-01 =
    .customer = Fennick, cellar keeper
    .complaint = I type my code wrong sometimes. Then right. After a week it locked me out of my own cellar.
    .remark-typos = Customer admits to fat fingers.
    .remark-first-thinking = Still has its first thinking, version 1.0.

lock4-02 =
    .customer = Moxa, jewel cutter
    .complaint = Somebody changed my code while the door was shut. I want my jewels back and a lock that locks.
    .remark-new-code = Opens fine with the new code. Customer did not set it.
    .remark-first-thinking = Still has its first thinking, version 1.0.

lock4-03 =
    .customer = Orrin, vault guard
    .complaint = A thief guessed codes all night and the lock never stopped him. It should give up after 3 tries.
    .remark-guard = Thief pressed C between every guess, says the guard.
    .remark-first-thinking = Still has its first thinking, version 1.0.

lock4-04 =
    .customer = Grandma Tuk, sweet shop
    .complaint = 12 years it worked. Now OPEN and SHUT both light up when it opens. Which one is it?
    .remark-sun = Sat in a sunny shop window for years. The sticker over the little glass window on its black brain peeled off.
    .remark-factory = Thinking is the factory version 1.0. Nobody changed it.

score5-01 =
    .customer = Brog, ball game judge
    .complaint = The home side had 99 points. They scored once more and the board went back to almost nothing. The home side cried. The guest side danced.
    .remark-scratches = Many scratches on the H key. Somebody pressed it a lot.

score5-02 =
    .customer = Yelli, hoop game coach
    .complaint = My player threw from far away and got 3 points. Then she threw from close by and got 3 again. Close throws give 1!
    .remark-three-pointer = Coach says the board was fine before the big tournament.

score5-03 =
    .customer = Dunt, stadium keeper
    .complaint = The guests score and the home side gets the points. The guests are very angry with me.
    .remark-grumpy-guests = Small bite marks on the case. Customer says it was a guest.

# Generated jobs share the messages below. The complaint is about the first
# test of the job. { $keys } are the keys it presses, { $key } is the last of
# them and { $expected } is what the display should show.

job-customer =
    .blib = Blib, eel herder
    .snorv = Snorv, moss baker
    .twee = Twee, egg counter
    .gorm = Gorm, bridge keeper
    .yalla = Yalla, rope weaver
    .krenn = Krenn, rain collector
    .pibbo = Pibbo, shell trader
    .murg = Murg, night watch
    .zazz = Zazz, spore farmer
    .oonk = Oonk, cart puller
    .vettle = Vettle, hive keeper
    .drubb = Drubb, pot mender
    .quilla = Quilla, star reader
    .hosk = Hosk, mud brick maker
    .nibbet = Nibbet, school helper
    .throm = Throm, ferry driver
    .pell = Pell, young courier (regular)
    .ommi = Old Ommi, sky counter (regular)
    .vosh = Inspector Vosh, city office (regular)
    .krell = Krell and Krull, twins (regular)

job-complaint-wrong-digit =
    .take-1 = I pressed { $keys } and the little window lied to me. It should say { $expected }.
    .take-2 = My cousin says { $keys } makes { $expected }. This box makes something else. One of them is wrong and it is not my cousin.
    .take-3 = It was a good box until it started to make mistakes. Press { $keys } and see. It should say { $expected }.

job-complaint-wake-wrong-digit =
    .take-1 = When I wake it up, it should say { $expected }. It says something else.
    .take-2 = It wakes up with the wrong numbers in the little window. It should say { $expected }.
    .take-3 = Every morning it wakes up confused. It should say { $expected } before I even touch it.

job-complaint-wrong-annunciator =
    .take-1 = I pressed { $keys } and the little marks next to the numbers lit up wrong. It should say { $expected }.
    .take-2 = The tiny lights at the side of the window do what they want. Press { $keys } and look. It should say { $expected }.
    .take-3 = After { $keys } it shows the wrong little mark. Now I cannot trust it. It should say { $expected }.

job-complaint-wake-wrong-annunciator =
    .take-1 = When I wake it up, a little mark next to the numbers lights up wrong. It should say { $expected }.
    .take-2 = It wakes up with the wrong tiny light at the side of the window. It should say { $expected }.
    .take-3 = It opens its eyes and already shows a mark it should not. It should say { $expected }.

job-complaint-no-response =
    .take-1 = I pressed { $keys }, but the { $key } button does nothing. It should say { $expected }.
    .take-2 = The { $key } button is deaf. Press { $keys } and the little window does not change. It should say { $expected }.
    .take-3 = I pressed { $keys }. The { $key } at the end changes nothing in the little window. It should say { $expected }.

job-complaint-freeze =
    .take-1 = I pressed { $keys } and it froze stiff. Only putting it to sleep helps.
    .take-2 = After { $keys } it stops listening to every button. I think it is sulking.
    .take-3 = Press { $keys } and it goes stiff like a cold eel. I have to put it to sleep and wake it up again.

job-complaint-wake-freeze =
    .take-1 = When I wake it up, it freezes stiff. No button helps.
    .take-2 = It wakes up and then plays dead. It does not listen to any button.
    .take-3 = It opens its eyes and goes stiff right away. I shook it. It did not help.

job-complaint-wrong-after-clear =
    .take-1 = I pressed { $keys }. The C button should wipe it clean, but it gets it wrong. It should say { $expected }.
    .take-2 = The C button does not forget properly. Press { $keys } and see. It should say { $expected }.
    .take-3 = After C it still remembers old things. Press { $keys }. It should say { $expected }.

job-remarks =
    .remark-sticky = Something sticky dried on the board, on the part that remembers numbers. It did not come off. The box still acts the same.
    .remark-spill = Customer spilled hot soup on it. Something sticky dried on the part that remembers numbers. It did not come off. The box acts strange since then.
    .remark-also-wrong-digit = Customer says other numbers come out wrong too.
    .remark-also-wrong-annunciator = Customer says the little marks are sometimes wrong too.
    .remark-also-no-response = Customer says a button sometimes does nothing.
    .remark-also-freeze = Customer says it sometimes freezes stiff too.
    .remark-also-wrong-after-clear = Customer says the C button does not always wipe it clean.

# Regulars come back three times in a run. Each visit tells a little more of
# their story. The ticket puts the visit in front of the complaint. The
# receipt prints the thanks when the device works. The last visit brings a
# gift.

regular-pell =
    .visit-1 = My boss gave me this box on my first day as a courier. I broke it on my first day too. Please do not tell him.
    .thanks-1 = My boss did not notice anything! I owe you a favor, human.
    .visit-2 = Boss made me head runner. Now I carry three boxes at once. One of them is sick again.
    .thanks-2 = The other runners ask where I get my boxes fixed. I tell them only the human knows.
    .visit-3 = Boss went to live by the warm sea. The courier shop is mine now. So is this broken box.
    .thanks-3 = A shop owner pays like a shop owner. The extra is for my first day.

regular-ommi =
    .visit-1 = I count the stars every night with this box. Last night it said there were fewer. That cannot be right.
    .thanks-1 = The stars are all back. Thank you, small human.
    .visit-2 = My grandson borrowed the box to count beetles. Now it counts wrong. Beetles are bad for boxes.
    .thanks-2 = It counts right again. I hid it from my grandson.
    .visit-3 = My eyes are old now. My grandson counts the stars for me. He sent me with this box and a little extra.
    .thanks-3 = We found a new star last night. We named it after you. The extra is from my grandson.

regular-vosh =
    .visit-1 = The city sent me to see if a human can really fix things. Fix this box. I will watch.
    .thanks-1 = It works. That proves nothing. I will be back.
    .visit-2 = Another box. A harder one, maybe. The city is still not sure about you.
    .thanks-2 = It works again. I wrote a good word about you in my report. Only one word.
    .visit-3 = This is my own box, not the city's. Please do not tell the office.
    .thanks-3 = The office says your shop may stay. I told Vresk to be nice to you. He laughed. The extra is from me.

regular-krell =
    .visit-1 = Krell: Krull says this box is his. It is mine. He dropped it. Fix it for me, not for him.
    .thanks-1 = Krell: It works! Krull: Because I brought it here.
    .visit-2 = Krell: We each have our own box now. Krull already broke his. This is his. Do not tell him I said so.
    .thanks-2 = Krull says thank you. Krell says Krull should be more careful.
    .visit-3 = Krell: We share one box again. It is easier to break only one. Krull: We broke it together.
    .thanks-3 = We agree on one thing now. The human is good. The extra is from both of us.
