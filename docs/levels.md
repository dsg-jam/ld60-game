# Levels

Levels live in `assets/levels`. Each level is a folder with three files:

- `level.ron`: the job, its tests and its firmware source.
- `buggy.asm`: the firmware the device comes in with.
- `fixed.asm`: a firmware that passes the tests.

## Add a level

1. Create a folder, for example `summa6/08-new-bug`, and write its `level.ron`.
   Give it a unique `id` and point `firmware` to a `.fw` source. See
   [Firmware language](dsl.md).
2. Run `cli gen` to write `buggy.asm` and `fixed.asm`.
3. Add the folder to `catalog.ron`. The game plays the levels in the order of
   this list.
4. Add a message named after the level `id` to `assets/locales/en/levels.ftl`.
   It needs these attributes:
   - `customer`: who brings the device in.
   - `complaint`: what the customer says is wrong.
   - `remark-<key>`: one for each key in `remarks` of `level.ron`.

   The attributes can use `$board` for the board of the job, like `6A`.

```ftl
summa6-08 =
    .customer = Grull, fish seller
    .complaint = It adds the fish twice.
    .remark-sticky = The 7 key sticks on board { $board }.
```

Run `cargo test -p ld60-levels -p ld60-game` to check that the level can be
solved and that all its text exists. Use `cli play <folder>` to try it.

## Generated jobs

`Generated::new` makes jobs from the working firmware of each device.
`Difficulty::for_job` sets how hard job number `n` is: the number of bugs, the
kinds of bugs, the chip, harmless RAM faults and how many keys show a bug. Later
jobs can instead have a single fault: a stuck RAM bit in a cell the working
firmware uses. The fix moves that cell to a free one.

Generated jobs share their text in `levels.ftl`:

- `job-customer`: one attribute for each name in `JobText::CUSTOMERS`.
- `job-complaint-<symptom>` and `job-complaint-wake-<symptom>`: the attributes
  `take-1` to `take-3` describe the first bug. They can use `$keys`, `$key` and
  `$expected`.
- `job-remarks`: the remarks for further bugs and faults.

To try one, run `cli gen --procedural <device> <seed> <n>`. It writes a level
folder and prints its path. Then run `cli play <folder>`.

## Add a regular

Regulars are locals who come back with one device after another and tell a
little more of their story each time. They bring generated jobs.

1. Add them to `regulars` in `catalog.ron`: an `id`, the job of the run `from`
   which they may come, and how many `visits` their story has.
2. Add their name as an attribute of `job-customer` in `levels.ftl`, named after
   the `id`.
3. Add a message `regular-<id>` with the attributes `visit-<n>` and `thanks-<n>`
   for each visit, counted from 1. The last visit brings a gift.

```ftl
regular-pell =
    .visit-1 = My boss gave me this box on my first day as a courier.
    .thanks-1 = My boss did not notice anything!
```
