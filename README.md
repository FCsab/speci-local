# speci-local

[![CI](https://github.com/FCsab/speci-local/actions/workflows/ci.yml/badge.svg)](https://github.com/FCsab/speci-local/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/FCsab/speci-local)](https://github.com/FCsab/speci-local/releases/latest)

An offline version of the two ELTE programming-course tools, in one native app:

- the **Specification Editor** ([progalap.elte.hu/specifikacio](https://progalap.elte.hu/specifikacio/)): write a specification, add test cases, see each one turn green or red;
- **Stuki**, the structogram editor ([progalap.elte.hu/stuki](https://progalap.elte.hu/stuki/)): draw structograms, generate them from a specification, and run them on the same test cases.

It needs no login and no network, so it keeps working when the university servers don't.
It runs on Windows, Linux, macOS and Android from one Rust code base ([egui](https://github.com/emilk/egui)).

## Contents

- [Install](#install)
- [Quick start](#quick-start)
- [Specifications](#specifications)
- [Stuki (structograms)](#stuki-structograms)
- [Files and sharing](#files-and-sharing)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Phones and Android](#phones-and-android)
- [Build from source](#build-from-source)
- [Development](#development)
- [Releasing](#releasing)
- [Not included](#not-included)
- [Credits](#credits)

## Install

Ready-made builds are attached to the [GitHub releases](https://github.com/FCsab/speci-local/releases).

| System | File | How |
| --- | --- | --- |
| Windows | `.zip` | Unzip and run the single executable. |
| Linux | `.tar.gz` | Unpack and run the single executable. |
| macOS | `speci-local-macos-universal.zip` | Unzip and move `speci-local.app` to Applications (Apple Silicon and Intel). |
| Android | `speci-local-android.apk` | Open it on the phone and allow installing from that source. |

Notes:

- **macOS**: the app isn't notarized by Apple, so the first start is blocked. Allow it in *System Settings → Privacy & Security → Open Anyway*, or run `xattr -dr com.apple.quarantine /Applications/speci-local.app`.
- **Linux**: needs the usual windowing libraries (X11/Wayland, `libxkbcommon`), and the Open/Save dialogs need `xdg-desktop-portal` (standard on GNOME and KDE).
  The first start installs a desktop entry (`~/.local/share/applications/speci-local.desktop`) and an icon (`~/.local/share/icons/hicolor/256x256/apps/speci-local.png`), so the app shows its icon and appears in the application menu. Delete those two files to remove it again.

## Quick start

1. Start the app. It opens with a sample specification that counts odd numbers:

   ```text
   Be:
   n∈N,
   szamok∈N[1..n]

   Ki:
   db∈N

   Ef:
   -

   Uf:
   db=DARAB(i=1..n, szamok[i] mod 2 = 1)
   ```

2. The test case next to it gives the inputs and the expected outputs as YAML:

   ```yaml
   n: 5
   szamok: [3, 72, 81, 4, 5]
   db: 3
   ```

3. Everything is evaluated as you type, in the background. The test tab turns green; change `db: 3` to `db: 2` and it turns red, with the value the specification gives.
4. Switch to the **Stuki** tab and press **Auto Stuki**: the counting algorithm is drawn as a structogram.
5. Press **Run tests**: the structogram is run on the same test case and gets its own green mark.

## Specifications

A specification is written in either block style, `Be:/Sa:/Ki:/Fv:/Ef:/Uf:` or `In:/Aux:/Out:/Fn:/Pre:/Post:`, with YAML test cases beside it.
Untick **Auto** to evaluate only when you press **Evaluate** (Ctrl+Enter).

### Same verdicts as the online editor

speci-local accepts and rejects the same specifications as the online editor and judges every test case the same way. This was checked against the online editor's own evaluator on 816 cases.

| | Supported |
| --- | --- |
| Types | elementary sets, intervals, arrays and matrices (with an initial value and any index range), records, type aliases, array literals with an index range (`[0..:1, 2]`) |
| Functions | one per `Fn:`/`Fv:`/`Def:` header, signature first |
| Logic | ∀/∃ and their word forms (`forall`/`minden`, `exists`/`létezik`, `in`/`eleme`), `true`/`igaz`, `false`/`hamis`, `->` |
| Arithmetic | `\|`, `∤`, `<>`, `mod`/`%`, `div`, `n!`, `abs`, `sqrt`, `round`/`kerekít`, `length`/`hossz`, `from`/`tól`, `to`/`ig` |
| Patterns | `SUM`, `COUNT`, `MAX`, `MIN`, `EXISTS`, `FORALL`, `SELECT`, `SEARCH`, `COPY`, `MIS`, `CONDMAX`, `CONDMIN` and their other names (`SZUMMA`, `MEGSZÁMOL`, `ELDÖNT`, `MIND`, `KERES`, `KIVÁLASZT`, `MÁSOL`/`MAP`, `KIVÁLOGAT`/`FILTER`, `MAXHA`, `MINHA`, …) |

It also rejects what the online editor rejects: `→`, `∧`/`∨`, `ABS`, `element`, `SELECT(i≥…)`, several functions in one block, outputs in the precondition, declared variables missing from the postcondition, and test cases that leave out a value or repeat a constant.

### Finding out why a test fails

- When a test case fails, the values the specification would give are computed and shown.
- When evaluation finds an error, its line is tinted in the editor. Click the location on an error card to jump there.
- The **Stepper** (pick it in the Evaluation pane) walks through the selected test case one reduction at a time:
  - the formula is shown with the part being evaluated highlighted, along with its value and the current loop variables;
  - `↓`/`↑` mark entering and leaving your `Fv:` functions, and inside a function its body becomes the formula shown;
  - step with ← / → or the buttons and slider. *Step list* shows every step at once; click one to jump to it.

### The editor

- Syntax highlighting, line numbers and context-aware completion: block headers at the start of a line, set names after `∈`, and the variables, types and functions you've declared.
- Patterns and quantifiers insert as snippets, e.g. `SUM` becomes `SUM(i=1..n, |)`. Use ↑↓ to choose, Tab or Enter to insert, Esc to close.
- Special characters: the symbol buttons, **Alt+1 … Alt+6** (**⌥1 … ⌥6** on macOS) for ∈ ∀ ∃ ≠ ≤ ≥, or type `forall`, `exists`, `leq` and let completion turn them into symbols.
- Light, dark or system theme, and an adjustable font size.

## Stuki (structograms)

Switch between the two editors with the **Specification** / **Stuki** tabs in the toolbar. Each tab keeps its own file, and New, Open, Save, Copy and Paste (and their shortcuts) work on the tab you're in.

### Editing

- **Elements**: drag a statement, decision, pre-test loop, post-test loop or multi-way decision onto a structogram.
  - Drop it on the upper or lower half of an element to put it before or after that element, or onto an empty statement `-` to replace it.
  - To put it after a loop or decision, drop it on that element's bottom edge (a line marks the spot), or just below the structogram if it's the last element.
  - Click an element to select it; the ↑ / ↓ buttons insert above or below the selected element.
- Texts are edited in place. Move an element by dragging its empty area or the handle in its corner. Delete it with the red ✕, the Delete key, or by dropping it outside the structograms.
- Hovering a multi-way decision shows **+C** / **+D** (add a case / the default case).
- Hovering a structogram shows **✕ F V R**: delete it, show the function header, the local declarations and the return value. The green **+** adds another structogram.
- **Undo / redo** with the buttons or Ctrl+Z / Ctrl+Y.

### Auto Stuki

**Auto Stuki** builds the structograms from the specification on the Specification tab, replacing the current ones (you're asked first if they have unsaved changes).

| In the specification | In the structogram |
| --- | --- |
| `x=…` in the postcondition | an assignment `x:=…` |
| `∀i∈[1..n]:(y[i]=…)` | a counting loop |
| a pattern (`SUM`, `COUNT`, `MAX`/`MIN`, `EXISTS`, `FORALL`, `SELECT`, `SEARCH`, `COPY`, `MIS`, `CONDMAX`/`CONDMIN`, under any of its names) or a quantifier | the standard algorithm taught at ELTE |
| a pattern inside another one, or inside a condition (`y[i]=DARAB(j=1..m, …)`, `∃i∈[1..n]:(SZUMMA(…)>0)`) | its own steps, computed first into a helper variable |
| `c -> x=…` | a decision; a matching `not c -> x=…`, or the other side of a comparison (`n>m` / `n≤m`), becomes its else branch |
| several `c -> x=…` that set the same output and aren't each other's opposite (`n>0`, `n=0`, `n<0`) | one multi-way decision, a case each |
| an `Fv:` function | a structogram of its own, with its header (`f(i:Egész):Logikai`) and return value |

- Loop variables and other helper variables are listed as local declarations, and the steps come in the order the outputs depend on each other. A nested loop that reuses its outer loop's variable gets a variable of its own (`i2`).
- Parts that only state a condition (like `db≤n`), and patterns used in a way that has no standard algorithm (like `MAX(…).maxval` inside a formula), can't become steps. They are kept as statements tinted red, for you to finish by hand. Parts like `k=k` that only mention a variable are left out.
- The texts follow the specification's language: `Be/Ki/Ef/Uf` gives `és`, `nem`, `igaz`, `Egész`, …; `In/Out/Pre/Post` gives `and`, `not`, `true`, `Integer`, ….

### Run tests

**Run tests** runs the structograms on the test cases of the Specification tab and shows a green or red mark for each one. It runs again after every change, to the structograms, the specification or the test cases.

A test case passes when the outputs the structogram computes from the test's inputs satisfy the postcondition. Hover a mark for the message and the computed outputs. Click a red mark to select the element where the run stopped.

The texts of the elements are read in the language of the specifications:

| Text | Meaning |
| --- | --- |
| `x:=…`, `t[i]:=…`, `r.field:=…`, `(a, b):=…` in a statement | an assignment |
| `i=1..n` at the top of a pre-test loop | a counting loop |
| anything else in a decision, loop or case | a condition (`i≤n és nem van`) |
| `-` or an empty statement | nothing |
| a structogram named like `f(i:Egész):Logikai` | a function: call it as `f(…)`, it gives back its return value |

The first structogram that isn't a function is the program. Functions of the specification that have no structogram are evaluated from their definition.
A run is stopped with a message when a text can't be read as a step, a variable is used before it has a value, an index is out of range, no case of a multi-way decision holds, or the program is still running after 2 million steps.

### Pictures and colors

- **Copying gives a picture**, ready to paste into a task, document or chat: the toolbar's copy button, **Copy image** or Ctrl+C (outside a text field) copy the selected structogram (the first one if none is selected) as a PNG. To copy a particular structogram, use the picture button in the bar above it or right-click it. **Export PNG** saves the picture as a file.
- Pictures are always light (white background, black lines), even in dark mode. Turn off *Light pictures* in the settings to get what's on screen instead.
- **Colors**: text color and text background for the selected part of a text (or all of it), element background and structogram background, from editable palettes saved with the project. **Colorize** colors the elements by kind.
- The sliders button holds the font and the web app's advanced settings: horizontal / vertical distances (linked or not), snapping, block height, spacing, border width and right padding. Zoom with the slider, Ctrl+wheel or a pinch.

## Files and sharing

Open a file with Ctrl+O, by dragging it onto the window, or from the command line (`speci-local file.sp`). Your session is also autosaved when you quit.

### `.sp`: a specification with its test cases

A `.sp` file is plain text: the specification, with each test case stored as a comment.

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

Because the test cases are comments, the whole file is still a valid specification. You can paste it into the ELTE editor as-is.

### `.st`: structograms

A `.st` file is the web app's own state as JSON (the `nodes` and `structograms` it keeps, plus the color palettes), so the same data moves between the two. Colored text is kept as the HTML the web app uses (`<span style="color: rgb(255, 0, 0);">…</span>`).

### One line for a whole project

- On the Specification tab, the copy button (Ctrl+Shift+C) puts the specification and all test cases on the clipboard as one compressed line, `SPECI1:…`. It's handy for sending through chat.
- On the Stuki tab, **Copy as text**, Ctrl+Shift+C or Shift-clicking the copy button copies one `STUKI1:…` line.
- Paste (Ctrl+Shift+V) reads either line and opens it on the right tab. It also accepts plain specification text, a `.st` file's text, and the web app's share links (`…/stuki/v1/?data=…`).
- If the system clipboard can't be read, a box opens where you can paste with Ctrl+V instead.

## Keyboard shortcuts

On macOS, use ⌘ instead of Ctrl and ⌥ instead of Alt.

| Shortcut | Action |
| --- | --- |
| Ctrl+N / Ctrl+O | New / open, on the current tab |
| Ctrl+S / Ctrl+Shift+S | Save / save as |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy / paste the whole project as one line |
| Ctrl+Enter | Evaluate the specification |
| Alt+1 … Alt+6 | Insert ∈ ∀ ∃ ≠ ≤ ≥ |
| ← / → | Stepper: previous / next step |
| Ctrl+Z / Ctrl+Y | Stuki: undo / redo |
| Ctrl+C | Stuki, outside a text field: copy the structogram as a picture |
| Delete | Stuki: remove the selected element |
| Ctrl+wheel | Stuki: zoom |

## Phones and Android

On a narrow screen (a phone, or a narrow window on the desktop), one pane is shown at a time, and the **Spec / Tests / Results** tabs switch between them. The settings menu holds *Save as…* and the theme.

- Android has no file dialogs. **Open** and **Save** show a list of the files in the app's folder (`Android/data/io.github.fcsab.specilocal/files/specifications`, which a computer sees over USB).
- To move a project between devices, use the copy / paste project buttons. They work with the Android clipboard, so the `SPECI1:…` line can be sent through any chat app.
- The on-screen keyboard types plain letters, digits and punctuation. Use the symbol buttons for ∈ ∀ ∃ ≠ ≤ ≥, or type `forall`, `exists`, `leq`. The paste button next to the symbol buttons pastes the clipboard at the cursor.
- Copying a structogram saves the picture into the app's folder, because Android's clipboard can't hold pictures from apps like this one.

## Build from source

You need a [Rust toolchain](https://rustup.rs) (edition 2024).

```sh
cargo run --release
```

On Debian or Ubuntu, install the build dependencies first:

```sh
sudo apt-get install libxkbcommon-dev libgtk-3-dev
```

On macOS, `scripts/bundle-macos.sh` builds `speci-local.app` (a universal binary for Apple Silicon and Intel) and zips it.

### Android

The Android app is the `android/` crate, a thin wrapper around the same library, built with [cargo-apk](https://github.com/rust-mobile/cargo-apk):

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-apk
export ANDROID_HOME=~/Android/Sdk ANDROID_NDK_ROOT=~/Android/Sdk/ndk/<version>
cargo apk run -p speci-local-android      # debug build, installed on the connected phone
```

Add `--target x86_64-linux-android` to build only for the emulator. The app logs to logcat (`adb logcat -s speci-local`).
`cargo apk build -p speci-local-android --release` needs a signing key: set `CARGO_APK_RELEASE_KEYSTORE` (a `.jks` file) and `CARGO_APK_RELEASE_KEYSTORE_PASSWORD`.

## Development

```sh
cargo test                                   # unit, UI and integration tests
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

CI runs all three on every push, builds the desktop app on Windows, Linux and macOS, and builds a debug APK.

| Path | What it holds |
| --- | --- |
| `src/parser/` | lexer, block splitter, grammar and static checks of specifications |
| `src/eval/` | the evaluator: values, operators, patterns, function calls, stepper trace |
| `src/data.rs`, `src/runner.rs` | reading YAML test cases, running them and explaining the verdict |
| `src/file.rs`, `src/compact.rs` | `.sp` files and the `SPECI1:` line |
| `src/editor/` | the code editor: highlighting and completion |
| `src/app/` | the window: toolbar, panes, results, stepper, dialogs, background evaluation |
| `src/stuki/` | the structogram editor: model, layout, canvas, colors, PNG and `.st` files |
| `src/stuki/from_spec.rs` | Auto Stuki |
| `src/stuki/run.rs` | running structograms on test cases |
| `src/platform.rs` | what differs between the desktop systems and Android |
| `android/` | the Android wrapper crate |
| `tests/` | case studies, edge cases, limits, and a fuzzer that damages valid documents |

How the pieces are tested against each other:

- `tests/cases.rs` holds case studies from the ELTE manual. Every specification that Auto Stuki can turn into steps completely is also run as a structogram, and its outputs must satisfy the postcondition.
- `tests/fuzz.rs` damages valid documents in thousands of small ways. The parser, the evaluator, Auto Stuki and the structogram runner must answer each with a verdict or a message, never a panic.
- The UI tests (`src/app/tests.rs`) drive the real window with [egui_kittest](https://crates.io/crates/egui_kittest).

`SKILL.md` is a manual of the specification language as the online editor defines it, usable as a skill for AI assistants.

## Releasing

1. Raise `version` in `Cargo.toml`. CI alone never publishes anything.
2. Press *Run workflow* on the Release workflow, or push a matching tag (`git tag v0.5.0 && git push origin v0.5.0`).

The workflow builds every platform, runs the tests, and attaches the files to the release.

For APKs that update in place, add the repository secrets `ANDROID_KEYSTORE` (the base64 of a `.jks` file) and `ANDROID_KEYSTORE_PASSWORD`. Without them, each release is signed with a new throwaway key, so the old version must be uninstalled before installing a new one.

## Not included

- From the Specification Editor: the history pane, and sharing by permalink.
- From Stuki: the INF login, saving to the university server, and fonts other than the bundled one (monospace or proportional).

## Credits

- The original web applications, the Specification Editor and Stuki, are made at the Faculty of Informatics of Eötvös Loránd University (ELTE IK). speci-local is an independent offline reimplementation and is not affiliated with the university.
- The UI uses 0xProto Nerd Font, with Noto Sans Math as the fallback for ∈ ∀ ∃. Both are embedded, and their licenses are in `assets/`.
