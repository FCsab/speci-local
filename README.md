# speci-local

An offline replacement for the ELTE Specification Editor (progalap.elte.hu/specifikacio), for when the university login is down.
It's a single native binary for Linux and Windows.

You write a specification (`Be:/Sa:/Ki:/Fv:/Ef:/Uf:` or `In:/Aux:/Out:/Fn:/Pre:/Post:`), add YAML test cases, and press **Evaluate** (Ctrl+Enter).
Each test tab turns green or red.

## Build & run

```sh
cargo run --release
```

Linux needs the usual windowing libraries (X11/Wayland, `libxkbcommon`). Windows needs no extra setup.
CI builds both platforms; download the binaries from the Actions artifacts.

## What it does

- Supports the full grammar from `SKILL.md`: elementary sets, intervals, arrays/matrices, records, type aliases, `Fn:` functions, ∀/∃, chained comparisons, `->`, `|`, `mod`/`%`,
  and the macros `SUM` (with filter), `COUNT`/`DARAB`, `MAX`, `MIN`, `EXISTS`, `FORALL`, `SELECT`, `SEARCH`, `COPY`, `MIS`, `CONDMAX`, `CONDMIN`.
- Checks the same rules as the ELTE editor (All Variables Mandate, reserved names such as `exists`, block order, header spelling), so a specification that's green here should also pass there.
- **Error messages are more detailed than the original's.** Each one shows the line, a caret, an explanation and a concrete fix. When a postcondition fails, it tells you which part failed and what the specification computes compared to what the test case expects (e.g. "`cnt`: the test case says 2, the specification gives 3").
- Outputs left out of a test case are computed and shown, which is handy when you're writing tests.
- An editor with syntax highlighting, line numbers and context-aware completion. It completes block headers at the start of a line, set names after `∈`, and the variables, types and functions you've declared. Theorem macros and quantifiers insert as snippets, e.g. `SUM` becomes `SUM(i=1..n, |)`. Typing `forall`, `exists` or `leq` inserts ∀ / ∃ / ≤. Use ↑↓ to choose, Tab or Enter to insert, Esc to close.
- When evaluation finds an error, its line is tinted in the editor. Click the location on an error card to jump there.
- Light, dark or system theme, and an adjustable font size.

## `.sp` files

Save with Ctrl+S (Ctrl+Shift+S for Save as) and open with Ctrl+O. You can also drag a file onto the window, or run `speci-local file.sp`; the file is evaluated as soon as it opens.
A `.sp` file is plain text: the specification, with each test case stored as a comment:

```text
// title: Odd numbers
Be:
n∈N,
szamok∈N[1..n]
...
/*--- test ---
n: 5
szamok: [3, 72, 81, 4, 5]
db: 3
---*/
```

Because the test cases are comments, the whole file is still a valid specification. You can paste it into the ELTE editor as-is. Your session is also autosaved when you quit.

## Not included

The step-by-step stepper and step list, the history pane, and sharing/permalinks.

## Fonts

The UI uses 0xProto Nerd Font, with Noto Sans Math as the fallback for ∈ ∀ ∃. Both are embedded, and their licenses are in `assets/`.
