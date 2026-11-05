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
- Save and open `.spec.json` files. The session is also autosaved when you quit.

## Not included

The step-by-step stepper and step list, the history pane, and sharing/permalinks.

## Fonts

The UI uses 0xProto Nerd Font, with Noto Sans Math as the fallback for ∈ ∀ ∃. Both are embedded, and their licenses are in `assets/`.
