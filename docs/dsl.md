# Firmware language

Level firmware is written in a small language and compiled to assembly. Each
statement becomes a short, fixed instruction pattern, so the output looks hand
written. Players only see the generated `.asm` files.

Sources live in `assets/levels` with the extension `.fw`. A level points to its
source in `level.ron`:

```ron
firmware: Some((source: "summa6/firmware.fw", switches: ["no-clear"])),
```

Run `cli gen` after you change a source. It rewrites `buggy.asm` and `fixed.asm`
of every level with a `firmware` field. `cli gen --check` and the levels tests
fail if a file is out of date.

The output is for the level's chip and board. The board is `board` in
`level.ron`, or board A if the field is missing. The game can move a level to
another board later. See the board revisions in the device docs, for example
[Tally 2](devices/tally2.md).

## Declarations

```
firmware "Tally-2 firmware 1.0" for tally2
cells tally[2] at M1    ; count, M2 is the rightmost digit
cell  last at M3        ; last key
const MINUS = 1
```

- `firmware` sets the title and the device. The board of the level gives the key
  codes.
- `cell` and `cells` name RAM. Arrays start at 1. The `;` text becomes the RAM
  map line in the header.
- A `;` line at the top level goes into the header. `#` comments are dropped.

## Blocks

| Form                  | Meaning                                                    |
| --------------------- | ---------------------------------------------------------- |
| `entry name { }`      | First block, at address 0                                  |
| `block name { }`      | Labeled code. Falls through to the next block.             |
| `routine name { }`    | Labeled code that ends in a return. Call it with `name()`. |
| `routine name(i) { }` | Same, but it reads `i`. Call it with `name(i)`.            |

Blocks keep their source order. A blank line goes before a block that is not
reached by falling through, except after the entry block. A block cannot fall
through into a routine, and `goto` cannot jump into a routine. Call it instead.

## Statements

`a` is the accumulator and `i` is the loop counter.

| Statement                                    | CalCo output                                              |
| -------------------------------------------- | --------------------------------------------------------- |
| `a = 5`, `a = x`, `a = arr[2]`, `a = arr[i]` | `LDI A, 5`, `LD Mn`, `LD Mn`, `LD [B+n]`                  |
| `x = a`, `arr[i] = a`, `x = y`               | `ST`, or `LD` and `ST`                                    |
| `a = KEY`, `ANN = a`                         | `IN KEY`, `OUT ANN`                                       |
| `ANN = ann(minus)`                           | `LDI A, n` and `OUT ANN`                                  |
| `a = x + 1`, `a += 1`, `a -= 1`              | `LD x` and `INC`, `INC`, `DEC`                            |
| `a = x + y + carry`, `a = x - y - carry`     | `LD x` and `ADC y` or `SBC y`                             |
| `a = bcd_add(x, y)`, `a = bcd_sub(x, y)`     | The above, then `DA` or `DS`                              |
| `carry = 0`, `carry = 1`, `i = 6`            | `CLC`, `SEC`, `LDI B, 6`                                  |
| `wait`, `da()`, `ds()`, `nop()`              | `WAIT`, `DA`, `DS`, `NOP`                                  |
| `repeat i as name { }`                       | Loop body, then `DJNZ name`                               |
| `for i = 6 downto 1 as name { }`             | `i = 6`, then the loop                                    |
| `name()`, `return`, `goto x`, `x:`           | `CALL`, `RET`, `JMP`, label                               |
| `if cond goto x`                             | Compare and jump                                          |
| `if cond { } else { }`                       | Compare, jump and generated labels                        |
| `match a { 'C' => goto x }`                  | `CPI` and `JZ` per arm. `0 to 9` gives `CPI 10` and `JC`. |
| `asm { }`                                    | Raw assembly lines                                        |

Loops run at least once, so `i` must start at 1 or more. A loop counts `i` down
and stops after the pass with `i = 1`.

Conditions are `a == v`, `a != v`, `a < v`, `a >= v`, `carry`, `!carry`, `zero`
and `!zero`. Key literals like `'+'` print in hex. Numbers keep the way you
wrote them. A `;` comment after a statement goes on its first instruction.

`ann(minus)` is the `ANN` value that lights the minus annunciator on the board
of the level. `ann(minus, error)` lights both. Use it instead of raw bits, so
the firmware stays right on every board.

The compiler does not optimize. It only drops a generated jump to the next line.

## Switches

One source holds the buggy and the fixed firmware of several levels:

```
bug "name" { fixed { ... } buggy { ... } }
fix "name" { before { ... } after { ... } }
change "name" { before { ... } after { ... } }
```

A switch can wrap declarations, statements or one value. Without a selection,
each switch takes its first side. When a level selects a name:

- `bug`: `buggy.asm` takes the `buggy` side.
- `fix`: `fixed.asm` takes the `after` side.
- `change`: both files take the `after` side.

All switches with the same name flip together and must be of the same kind. An
unknown name is an error.

## Chip revisions

The level's chip picks what the compiler may use. If the chip has no `CALL`, a
routine is copied into each call site. If it has no `DJNZ` or no `[B+n]`, a loop
with a known count of 1 to 16 is unrolled. Anything else is an error that names
the revision.

The compiler also checks that loops do not nest, that a call in a loop keeps
`i`, and that calls fit the stack. These are errors for `fixed.asm` and warnings
for `buggy.asm`, where the broken rule is often the bug.

## Other instruction sets

Each instruction set has a backend in `packages/dsl/src/backend/`. The backend
names cells, flags and ports, wraps indexed addresses, prints instructions and
lowers each statement.

The compiler targets accumulator machines with one index register that also
counts loops. `a` is the accumulator and `i` is the index register. A machine of
another shape, like a stack machine or one with several general registers, needs
changes to the compiler first, not only a new backend.

## Example

```
firmware "Tally-2 firmware 1.0" for tally2

cells tally[2] at M1    ; count, M2 is the rightmost digit
cell  last at M3        ; last key

entry reset {
    a = 0
    tally[1] = a
    tally[2] = a
}

block main {
    wait
    a = KEY
    last = a
    match a {
        'C' => goto reset
        '+' => goto count
    }
    goto main
}

block count {
    a = tally[2] + 1
    bug "double-count" {
        buggy {
            a += 1
        }
    }
    if a < 10 goto ones
    a = 0
    tally[2] = a
    tens()
    goto main
}

block ones {
    tally[2] = a
    goto main
}

routine tens {
    a = tally[1] + 1
    if a >= 10 {
        a = 0
    }
    tally[1] = a
}
```

The fixed output for a Rev B chip on board 2A starts like this:

```
; Tally-2 firmware 1.0
;
; M1-M2  count, M2 is the rightmost digit
; M3     last key

reset:  LDI  A, 0
        ST   M1
        ST   M2
main:   WAIT
        IN   KEY
        ST   M3
        CPI  0x0D
        JZ   reset
```
