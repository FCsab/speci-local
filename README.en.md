# speci-local

[Magyar](README.md) | **English**

[![CI](https://github.com/FCsab/speci-local/actions/workflows/ci.yml/badge.svg)](https://github.com/FCsab/speci-local/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/FCsab/speci-local)](https://github.com/FCsab/speci-local/releases/latest)

An offline version of the two ELTE programming-course tools, in one native app:

- the **Specification Editor** ([progalap.elte.hu/specifikacio](https://progalap.elte.hu/specifikacio/)): write a specification, add test cases, see each one turn green or red;
- **Stuki**, the structogram editor ([progalap.elte.hu/stuki](https://progalap.elte.hu/stuki/)): draw structograms and run them on the same test cases.

It needs no login and no network, so it keeps working when the university servers don't.
It runs on Windows, Linux, macOS and Android from one Rust code base ([egui](https://github.com/emilk/egui)).

> **The project is under heavy development, with new versions coming out every day.**

## Contents

- [Install](#install)
- [Antivirus warnings](#antivirus-warnings)
- [Quick start](#quick-start)
- [What's new in v1.0](#whats-new-in-v10)
- [Specifications](#specifications)
- [Stuki (structograms)](#stuki-structograms)
- [Files and sharing](#files-and-sharing)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Phones and Android](#phones-and-android)
- [Build from source](#build-from-source)
- [Development](#development)
- [Releasing](#releasing)
- [Privacy](#privacy)
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
- **Linux**: needs OpenGL and the usual windowing libraries (X11/Wayland, `libxkbcommon`), and the Open/Save dialogs need `xdg-desktop-portal` (standard on GNOME and KDE).
  The first start installs a desktop entry (`~/.local/share/applications/speci-local.desktop`) and an icon (`~/.local/share/icons/hicolor/256x256/apps/speci-local.png`), so the app shows its icon and appears in the application menu. Delete those two files to remove it again.

## Antivirus warnings

The released programs aren't code-signed, so Windows SmartScreen or an antivirus program may warn about them or even quarantine them.
That is common for small unsigned apps that few people have downloaded yet, and it doesn't mean anything harmful was found.
On Windows, *More info → Run anyway* starts the app.

Every release is built from this repository by the [Release workflow](.github/workflows/release.yml) on GitHub's own machines.
If you still don't feel safe running a downloaded program, build it yourself from the source code: see [Build from source](#build-from-source). It takes a few minutes, and the result is the same app.

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
4. Switch to the **Stuki** tab and draw the counting algorithm as a structogram.
5. Press **Run tests**: the structogram is run on the same test case and gets its own green mark.

## What's new in v1.0

In v1.0.1:

- **Open in web editor** and **Copy web editor link** in the settings menu open the current specification or structograms in the ELTE web editor; Paste also reads the specification editor's `?data=` links.
- A function whose body is a pattern with several results (like `f:N->N, f(i)=FELTMAX(…)`) is now an error, as it is in the web editor; take one part with `.3`, or list every part in the codomain.
- Stuki: the move and delete buttons are block-height squares, the "+" tile is a circle, and empty statements (`-`) can no longer be dragged out of loops and branches.
- The first start offers to add the app to the application menu (Linux) or the Start menu (Windows).

Key improvements and changes since v0.8.0:

- **Bilingual documentation**: Full documentation is now available in Hungarian (`README.md`) and English (`README.en.md`).
- **Editor improvements**:
  - Typing `<=`, `>=`, and `!=` in the specification editor automatically converts them to `≤`, `≥`, `≠`.
  - Multi-line YAML test data: arrays and records can now span multiple rows cleanly.
  - Smart indentation: pressing Enter in the test case editor automatically indents based on open brackets and braces.
  - Automatic file renaming: saving a specification whose task title changed automatically renames the `.sp` file if it was named after the task.
- **Smaller size and lighter performance**:
  - Binary sizes and memory footprint have been significantly reduced (~12 MB on Linux, ~11 MB on Windows; downloads ~5 MB).
  - Linux now uses eframe's OpenGL renderer instead of wgpu for lower memory usage and smaller binaries.
  - Embedded fonts are subsetted, and release builds use Link-Time Optimization (LTO).
- **Internal cleanups**: Unified workspace dependency versions in `Cargo.toml` and cleaner codebase structure.

## Specifications

A specification is written in either block style, `Be:/Sa:/Ki:/Fv:/Ef:/Uf:` or `In:/Aux:/Out:/Fn:/Pre:/Post:`, with YAML test cases beside it.
Untick **Auto** to evaluate only when you press **Evaluate** (Ctrl+Enter).

### Same verdicts as the online editor

speci-local accepts and rejects the same specifications as the online editor and judges every test case the same way. This was checked against the online editor's own evaluator on 816 cases.

| | Supported |
| --- | --- |
| Types | elementary sets, intervals, arrays and matrices (with an initial value and any index range), records, type aliases, array literals with an index range (`[0..:1, 2]`) |
| Functions | one per `Fn:`/`Fv:`/`Def:` header, signature first |
| Logic | ∀/∃ and their word forms (`forall`/`minden`, `exists`/`létezik`, `in`/`eleme`), `true`/`igaz`, `false`/`hamis`, `->`, cases `{a, ha c; b egyébként}` |
| Arithmetic | `\|`, `∤`, `<>`, `mod`/`%`, `div`, `n!`, `abs`, `sqrt`, `round`/`kerekít`, `length`/`hossz`, `from`/`tól`, `to`/`ig` |
| Test data | YAML; an array or record may span several rows, and Enter in the test case editor indents like a JSON editor |
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
- Special characters: the symbol buttons, **Alt+1 … Alt+6** (**⌥1 … ⌥6** on macOS) for ∈ ∀ ∃ ≠ ≤ ≥, or type `forall`, `exists`, `leq` and let completion turn them into symbols. `<=`, `>=` and `!=` typed by hand become ≤ ≥ ≠.
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

### `.sp`: the whole project in one file

A project is one `.sp` file: the specification, its test cases and the structograms of the Stuki tab. Save, Open and New work on all three at once, from either tab. The file is plain text: the specification, with each test case and the structograms stored as comments.

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

/*--- stuki ---
{
  "nodes": { … },
  "structograms": [ … ],
  …
}
---*/
```

Because the test cases and the structograms are comments, the whole file is still a valid specification. You can paste it into the ELTE editor as-is. The structograms are the web Stuki's JSON (see below). A project whose Stuki tab was never touched has no `stuki` block.

A file named after its task follows the task: rename the task, save, and the file gets the new name (unless another file already has it).

### `.st`: structograms from the web Stuki

Opening a `.st` file (or dropping it on the window) reads its structograms into the current project in place of the ones there; Ctrl+Z on the Stuki tab brings those back, and the next save writes them into the `.sp` file. A `.st` file is the web app's own state as JSON (the `nodes` and `structograms` it keeps, plus the color palettes), so the same data moves between the two. Colored text is kept as the HTML the web app uses (`<span style="color: rgb(255, 0, 0);">…</span>`).

### One line for a whole project

- On the Specification tab, the copy button (Ctrl+Shift+C) puts the whole project (the specification, all test cases and the structograms) on the clipboard as one compressed line, `SPECI1:…`. It's handy for sending through chat.
- On the Stuki tab, **Copy as text**, Ctrl+Shift+C or Shift-clicking the copy button copies only the structograms, as one `STUKI1:…` line.
- **Open in web editor** in the settings menu opens the current tab's project in the ELTE web editor; **Copy web editor link** copies that link instead. The web editor has no title, so a specification's title goes in as its first comment line.
- Paste (Ctrl+Shift+V) reads either line and opens it on the right tab: a `SPECI1:` line replaces the whole project, a `STUKI1:` line only the structograms (Ctrl+Z undoes that). It also accepts plain specification text, a `.st` file's text, and the web apps' share links (`…/specifikacio/v1/?data=…`, `…/stuki/v1/?data=…`).
- If the system clipboard can't be read, a box opens where you can paste with Ctrl+V instead.

## Keyboard shortcuts

On macOS, use ⌘ instead of Ctrl and ⌥ instead of Alt.

| Shortcut | Action |
| --- | --- |
| Ctrl+N / Ctrl+O | New / open project |
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

You need [Rust](https://rustup.rs) (stable, edition 2024) and the source code: either `git clone https://github.com/FCsab/speci-local`, or *Source code (zip)* from a [release](https://github.com/FCsab/speci-local/releases), unpacked.
Run the build commands in the source folder. The first build downloads the libraries it needs and takes a few minutes; `cargo run --release` builds and starts the app in one step.

### Windows

1. Install Rust with [rustup-init.exe](https://rustup.rs) (or `winget install Rustlang.Rustup`). When it asks, let it install the Visual Studio C++ build tools too: Rust needs their linker.
2. Open a new PowerShell window in the source folder and build:

   ```powershell
   cargo build --release
   ```

3. The program is `target\release\speci-local.exe`.

### Linux

1. Install a C compiler and the libraries the window and the file dialogs build against:

   ```sh
   sudo apt-get install build-essential curl libxkbcommon-dev libgtk-3-dev   # Debian, Ubuntu
   sudo dnf install gcc libxkbcommon-devel gtk3-devel                        # Fedora
   sudo pacman -S base-devel libxkbcommon gtk3                               # Arch
   ```

2. Install Rust:

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Open a new terminal in the source folder and build:

   ```sh
   cargo build --release
   ```

4. The program is `target/release/speci-local`.

### macOS

1. Install Apple's command line tools (compiler and linker):

   ```sh
   xcode-select --install
   ```

2. Install Rust:

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Open a new terminal in the source folder and build:

   ```sh
   cargo build --release                # the program: target/release/speci-local
   scripts/bundle-macos.sh              # or a whole app: target/speci-local.app (and a .zip of it)
   ```

   An app you built yourself isn't quarantined, so macOS starts it without the *Open Anyway* step.

### Nix

```sh
nix run github:FCsab/speci-local            # run without installing
nix profile install github:FCsab/speci-local
nix develop                                 # shell with cargo, clippy, rustfmt, rust-analyzer
```

Or add `inputs.speci-local.url = "github:FCsab/speci-local";` to a NixOS / Home Manager flake and use `speci-local.packages.${system}.default`.
The flake reads the version from `Cargo.toml` and the crate hashes from `Cargo.lock`, so it needs no edits when either changes; `flake.lock` (nixpkgs) is bumped by a weekly pull request.

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
| `src/file.rs`, `src/compact.rs` | `.sp` project files and the `SPECI1:` line |
| `src/editor/` | the code editor: highlighting, completion and Vim keys (`vim/`) |
| `src/app/` | the window: toolbar, panes, results, stepper, dialogs, background evaluation |
| `src/stuki/` | the structogram editor: model, layout, canvas, colors, PNG, and the web Stuki's JSON (`.st` files and the `stuki` block of `.sp` files) |
| `src/stuki/run/` | running structograms on test cases: compiler and machine |
| `src/platform.rs` | what differs between the desktop systems and Android |
| `android/` | the Android wrapper crate |
| `tests/` | case studies, edge cases, limits, and a fuzzer that damages valid documents |

How the pieces are tested against each other:

- `tests/cases.rs` holds case studies from the ELTE manual.
- `tests/fuzz.rs` damages valid documents in thousands of small ways. The parser, the evaluator and the structogram runner must answer each with a verdict or a message, never a panic.
- The UI tests (`src/app/tests.rs`) drive the real window with [egui_kittest](https://crates.io/crates/egui_kittest).

`SKILL.md` is a manual of the specification language as the online editor defines it, usable as a skill for AI assistants.

## Releasing

1. Raise `version` in `Cargo.toml`. CI alone never publishes anything.
2. Press *Run workflow* on the Release workflow, or push a matching tag (`git tag v1.0.0 && git push origin v1.0.0`).

The workflow builds every platform, runs the tests, and attaches the files to the release.

For APKs that update in place, add the repository secrets `ANDROID_KEYSTORE` (the base64 of a `.jks` file) and `ANDROID_KEYSTORE_PASSWORD`. Without them, each release is signed with a new throwaway key, so the old version must be uninstalled before installing a new one.

## Privacy

The app sends nothing anywhere unless you ask it to. The one exception is the update check: at start the app asks GitHub (`api.github.com`) for the number of the latest release, and sends nothing else. Turn it off with *Don't show again* in the update dialog.

## Not included

- From the Specification Editor: the history pane, and sharing by permalink.
- From Stuki: the INF login, saving to the university server, and fonts other than the bundled one (monospace or proportional).

## Credits

- The original web applications, the Specification Editor and Stuki, are made at the Faculty of Informatics of Eötvös Loránd University (ELTE IK). speci-local is an independent offline reimplementation and is not affiliated with the university.
- The UI uses 0xProto Nerd Font, with Noto Sans Math as the fallback for ∈ ∀ ∃. Both are embedded, and their licenses are in `assets/`.
