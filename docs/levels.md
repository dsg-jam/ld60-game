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
