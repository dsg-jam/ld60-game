# Notes from the human who ran the shop before, one for each tutorial job.

tutorial-footer = NOTES FROM MARTA

tutorial-1 =
    .title = To whoever sits at this bench next
    .label = FIRST JOB
    .welcome = The locals bring us old human gadgets. They pay well to have them fixed.
    .ticket = 1. Read the job ticket. It says what the gadget does wrong.
    .open = 2. Use "{ click-device }" and turn it over. Take out the screws and lift off the back cover.
    .cable = 3. Click the black header on the board to plug in the computer's cable.
    .computer = 4. Use the computer. Press F8 to dump the firmware off the gadget, then F8 again to disassemble it.
    .fix = 5. Find the wrong line and change it.
    .flash = 6. Press F6 FLASH to put your fix into the gadget, then use "{ click-device }" and try its buttons.
    .finish = 7. If it works, pull the cable, put the cover and the screws back, and stamp the ticket.
    .idea = On this gadget, LDI A, 5 puts 5 in A. ST M2 shows A on its digit.

tutorial-2 =
    .title = Lines
    .label = LINES
    .idea = The chip runs the lines from the top down, one after the other.
    .inc = INC adds 1 to A. LD M2, INC, ST M2 takes the right digit, adds 1 and shows it again.

tutorial-3 =
    .title = Labels and jumps
    .label = LABELS
    .idea = A word with a colon, like count:, names a line.
    .jump = JMP count goes back to that line and runs on from there.
    .wait = WAIT waits there until the next key press.

tutorial-4 =
    .title = Keys
    .label = KEYS
    .idea = This gadget has two keys. IN KEY puts the code of the pressed key in A.
    .manual = The manual of each gadget in the SHOP MANUALS file lists the code of each key.
    .datasheet = The datasheet of the chip, in the same file, says what each line does.

tutorial-5 =
    .title = Compares
    .label = COMPARES
    .idea = CPI 13 compares A with 13. A does not change.
    .jump = JZ start then goes to start if A was 13.

tutorial-6 =
    .title = Lower than
    .label = LOWER
    .idea = After a compare, JC jumps if A was lower.
    .digit = A digit only goes up to 9. A 10 in a digit shows as an angry squiggle.

tutorial-7 =
    .title = Keeping the shop
    .label = THE SHOP
    .versions = F3 VERSIONS keeps your earlier tries. F2 NOTES is a page for your own notes.
    .rent = After this customer, every visit takes time. When the ringed planet sets, the landlord comes for the rent.
    .trader = After the rent, a scrap trader leaves a crate on the counter. Buy a gadget there, and the locals start to bring that kind too.
