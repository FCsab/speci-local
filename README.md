# speci-local

An offline replacement for the ELTE Specification Editor (progalap.elte.hu/specifikacio), for when the university login is down.
It runs natively on Windows, Linux, macOS and Android, from one Rust code base ([egui](https://github.com/emilk/egui)).

You write a specification (`Be:/Sa:/Ki:/Fv:/Ef:/Uf:` or `In:/Aux:/Out:/Fn:/Pre:/Post:`) and add YAML test cases.
Everything is re-evaluated automatically as you type, in the background, and each test tab turns green or red.
Untick **Auto** if you'd rather evaluate manually with **Evaluate** (Ctrl+Enter).

The **Stuki** tab is an offline version of the ELTE structogram editor ([progalap.elte.hu/stuki](https://progalap.elte.hu/stuki/)): see [Stuki (structograms)](#stuki-structograms).

## Build & run

```sh
cargo run --release
```

On Linux, the first start installs a desktop entry (`~/.local/share/applications/speci-local.desktop`) and an icon (`~/.local/share/icons/hicolor/256x256/apps/speci-local.png`). Wayland window managers need these to show the app's icon, and they also put speci-local in your application menu. Delete those two files to remove it again.

Linux needs the usual windowing libraries (X11/Wayland, `libxkbcommon`), and the Open/Save dialogs need `xdg-desktop-portal` (standard on GNOME and KDE). Windows and macOS need no extra setup.

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

## Downloads

Ready-made builds are attached to the [GitHub releases](https://github.com/FCsab/speci-local/releases):

- Linux (`.tar.gz`) and Windows (`.zip`): a single executable.
- macOS (`speci-local-macos-universal.zip`): unzip and move `speci-local.app` to Applications. The app isn't notarized by Apple, so the first start is blocked. Allow it in *System Settings → Privacy & Security → Open Anyway*, or run `xattr -dr com.apple.quarantine /Applications/speci-local.app`.
- Android (`speci-local-android.apk`): open it on the phone and allow installing from that source.

To publish a new release, raise `version` in `Cargo.toml` (CI alone never publishes anything), then press *Run workflow* on the Release workflow, or push a matching tag (`git tag v0.4.0 && git push origin v0.4.0`).
It builds every platform, runs the tests, and attaches the files.
For APKs that update in place, add the repository secrets `ANDROID_KEYSTORE` (the base64 of a `.jks` file) and `ANDROID_KEYSTORE_PASSWORD`.
Without them, each release is signed with a new throwaway key, so you must uninstall the old version before installing a new one.

## On phones

On a narrow screen (a phone, or a narrow window on the desktop), one pane is shown at a time, and the **Spec / Tests / Results** tabs switch between them.
The settings menu holds *Save as…* and the theme.

Android has no file dialogs. **Open** and **Save** show a list of the specifications in the app's folder (`Android/data/io.github.fcsab.specilocal/files/specifications`, which a computer sees over USB).
To move a project between devices, use the copy / paste project buttons. They work with the Android clipboard, so the `SPECI1:…` line can be sent through any chat app.
The paste button next to the symbol buttons pastes the clipboard at the cursor.

The on-screen keyboard types plain letters, digits and punctuation. Use the symbol buttons for ∈ ∀ ∃ ≠ ≤ ≥, or type `forall`, `exists`, `leq` and let completion turn them into symbols.

## What it does

- Accepts and rejects the same specifications as the online editor, and judges every test case the same way (checked against the online editor's own evaluator on 816 cases):
  - elementary sets, intervals, arrays/matrices (also with an initial value and any index range), records, type aliases, array literals with an index range (`[0..:1, 2]`);
  - functions, one per `Fn:`/`Fv:`/`Def:` header (signature first);
  - ∀/∃ and their word forms (`forall`/`minden`, `exists`/`létezik`, `in`/`eleme`), `true`/`igaz`, `false`/`hamis`, `->`, `|`, `∤`, `<>`, `mod`/`%`, `div`, `n!`, and `abs`, `sqrt`, `round`/`kerekít`, `length`/`hossz`, `from`/`tól`, `to`/`ig`;
  - the patterns `SUM`, `COUNT`, `MAX`, `MIN`, `EXISTS`, `FORALL`, `SELECT`, `SEARCH`, `COPY`, `MIS`, `CONDMAX`, `CONDMIN` and their other names (`SZUMMA`, `MEGSZÁMOL`, `ELDÖNT`, `MIND`, `KERES`, `KIVÁLASZT`, `MÁSOL`/`MAP`, `KIVÁLOGAT`/`FILTER`, `MAXHA`, `MINHA`, …).
- Rejects what the online editor rejects: `→`, `∧`/`∨`, `ABS`, `element`, `SELECT(i≥…)`, several functions in one block, outputs in the precondition, declared variables missing from the postcondition, and test cases that leave out a value or repeat a constant.
- When a test case fails, the values the specification would give are computed and shown.
- An editor with syntax highlighting, line numbers and context-aware completion. It completes block headers at the start of a line, set names after `∈`, and the variables, types and functions you've declared. Theorem macros and quantifiers insert as snippets, e.g. `SUM` becomes `SUM(i=1..n, |)`. Typing `forall`, `exists` or `leq` inserts ∀ / ∃ / ≤. Use ↑↓ to choose, Tab or Enter to insert, Esc to close.
- When evaluation finds an error, its line is tinted in the editor. Click the location on an error card to jump there.
- **Stepper** (as in the original): pick *Stepper* in the Evaluation pane to walk through the selected test case one reduction at a time.
  - The formula is shown with the part being evaluated highlighted, along with its value and the current loop variables.
  - `↓`/`↑` mark entering and leaving your `Fv:` functions; inside a function, its body becomes the formula shown.
  - Step with ← / → or the buttons and slider. *Step list* shows every step at once; click one to jump to it.
- **Copy / paste a whole project.** The copy button (Ctrl+Shift+C) puts the spec and all test cases on the clipboard as one compact line (`SPECI1:…`, compressed). It's handy for sending through chat.
  - The paste button (Ctrl+Shift+V) turns that line, or plain specification text, back into the project.
  - If the system clipboard can't be read, a box opens where you can paste with Ctrl+V instead.
- Special characters: the symbol buttons or **Alt+1 … Alt+6** (**⌥1 … ⌥6** on macOS) for ∈ ∀ ∃ ≠ ≤ ≥ (in that order).
- Light, dark or system theme, and an adjustable font size.

## `.sp` files

Save with Ctrl+S (Ctrl+Shift+S for Save as) and open with Ctrl+O (⌘ instead of Ctrl on macOS). You can also drag a file onto the window, or run `speci-local file.sp`; the file is evaluated as soon as it opens.
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

## Stuki (structograms)

Switch between the two editors with the **Specification** / **Stuki** tabs in the toolbar. Each tab keeps its own file, and New, Open, Save, Copy and Paste (and their shortcuts) work on the tab you're in.

- **Elements**: drag a statement, decision, pre-test loop, post-test loop or multi-way decision onto a structogram. Drop it on the upper or lower half of an element to put it before or after that element, or onto an empty statement `-` to replace it. To put it after a loop or decision, drop it on the bottom edge of the loop or decision (a line marks the spot), or just below the structogram if it's the last element. Click an element to select it; the ↑ / ↓ buttons insert above or below the selected element.
- Texts are edited in place. Move an element by dragging its empty area or the handle in its corner, and delete it with the red ✕ or by dropping it outside the structograms (Delete also removes the selected element).
- Hovering a multi-way decision shows **+C** / **+D** (add a case / the default case). Hovering a structogram shows **✕ F V R**: delete it, show the function header, the local declarations and the return value. The green **+** adds another structogram.
- **Colors**: text color and text background for the selected part of a text (or all of it), element background and structogram background, from editable palettes saved with the project. **Colorize** colors the elements by kind.
- The sliders button holds the font and the web app's advanced settings: horizontal / vertical distances (linked or not), snapping, block height, spacing, border width and right padding. Zoom with the slider, Ctrl+wheel or a pinch.
- **Copying gives a picture**, ready to paste into a task, document or chat: the toolbar's copy button, **Copy image** or Ctrl+C (outside a text field) copy the selected structogram (the first one if none is selected) as a PNG. To copy a particular structogram, use the picture button in the bar above it or right-click it. **Export PNG** saves the picture as a file. Pictures are always light (white background, black lines), even in dark mode; turn off *Light pictures* in the settings to get what's on screen instead. On Android, copying saves the picture into the app's folder, because Android's clipboard can't hold pictures from apps like this one.
- **Undo / redo** with the buttons or Ctrl+Z / Ctrl+Y.
- **Copy / paste a whole project** like on the Specification tab: **Copy as text**, Ctrl+Shift+C or Shift-clicking the copy button copies one `STUKI1:…` line. Paste also accepts a `.st` file's text and the web app's share links (`…/stuki/v1/?data=…`). Paste picks the right tab on its own: a `SPECI1:` line opens on the Specification tab and a `STUKI1:` line on the Stuki tab.

### `.st` files

A `.st` file is the web app's own state as JSON (the `nodes` and `structograms` it keeps, plus the color palettes), so the same data moves between the two. Colored text is kept as the HTML the web app uses (`<span style="color: rgb(255, 0, 0);">…</span>`). Open `.st` files like `.sp` files: Ctrl+O, dragging them onto the window, or `speci-local file.st`.

Not included from the web app: the INF login, saving to the university server, and fonts other than the bundled one (monospace or proportional).

## Not included

The step-by-step stepper and step list, the history pane, and sharing/permalinks.

## Fonts

The UI uses 0xProto Nerd Font, with Noto Sans Math as the fallback for ∈ ∀ ∃. Both are embedded, and their licenses are in `assets/`.
