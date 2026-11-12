# speci-local

**Magyar** | [English](README.en.md)

[![CI](https://github.com/FCsab/speci-local/actions/workflows/ci.yml/badge.svg)](https://github.com/FCsab/speci-local/actions/workflows/ci.yml)
[![Legújabb kiadás](https://img.shields.io/github/v/release/FCsab/speci-local)](https://github.com/FCsab/speci-local/releases/latest)

Az ELTE két programozás-oktatási eszközének offline változata, egyetlen natív alkalmazásban:

- a **Specifikációszerkesztő** ([progalap.elte.hu/specifikacio](https://progalap.elte.hu/specifikacio/)): specifikáció írása, tesztesetek hozzáadása, és mindegyikről azonnal látszik, hogy zöld vagy piros;
- a **Stuki** struktogramszerkesztő ([progalap.elte.hu/stuki](https://progalap.elte.hu/stuki/)): struktogramok rajzolása és futtatása ugyanazokon a teszteseteken.

Nem kell hozzá bejelentkezés és hálózat sem, így akkor is működik, amikor az egyetemi szerverek nem.
Windowson, Linuxon, macOS-en és Androidon fut, egyetlen Rust kódbázisból ([egui](https://github.com/emilk/egui)).

> **A projekt aktív fejlesztés alatt áll, naponta jelennek meg új verziók.**

## Tartalom

- [Telepítés](#telepítés)
- [Gyors kezdés](#gyors-kezdés)
- [Specifikációk](#specifikációk)
- [Stuki (struktogramok)](#stuki-struktogramok)
- [Fájlok és megosztás](#fájlok-és-megosztás)
- [Billentyűparancsok](#billentyűparancsok)
- [Telefonok és Android](#telefonok-és-android)
- [Fordítás forráskódból](#fordítás-forráskódból)
- [Fejlesztés](#fejlesztés)
- [Kiadás](#kiadás)
- [Ami nem került bele](#ami-nem-került-bele)
- [Köszönet](#köszönet)

## Telepítés

A kész változatok a [GitHub-kiadásoknál](https://github.com/FCsab/speci-local/releases) tölthetők le.

| Rendszer | Fájl | Teendő |
| --- | --- | --- |
| Windows | `.zip` | Kicsomagolás, majd az egyetlen futtatható fájl elindítása. |
| Linux | `.tar.gz` | Kicsomagolás, majd az egyetlen futtatható fájl elindítása. |
| macOS | `speci-local-macos-universal.zip` | Kicsomagolás, majd a `speci-local.app` áthúzása az Alkalmazások mappába (Apple Silicon és Intel). |
| Android | `speci-local-android.apk` | Megnyitás a telefonon, és a telepítés engedélyezése ebből a forrásból. |

Megjegyzések:

- **macOS**: az alkalmazást az Apple nem hitelesítette (notarization), ezért az első indítást letiltja. Engedélyezd a *Rendszerbeállítások → Adatvédelem és biztonság → Megnyitás mindenképp* pontban, vagy futtasd ezt: `xattr -dr com.apple.quarantine /Applications/speci-local.app`.
- **Linux**: a szokásos ablakkezelő könyvtárak kellenek hozzá (X11/Wayland, `libxkbcommon`), a Megnyitás/Mentés ablakokhoz pedig az `xdg-desktop-portal` (GNOME-on és KDE-n alapból megvan).
  Az első indítás telepít egy asztali bejegyzést (`~/.local/share/applications/speci-local.desktop`) és egy ikont (`~/.local/share/icons/hicolor/256x256/apps/speci-local.png`), így az alkalmazásnak lesz ikonja, és megjelenik az alkalmazásmenüben. Ha törlöd ezt a két fájlt, eltűnik onnan.

## Gyors kezdés

1. Indítsd el az alkalmazást. Egy mintaspecifikációval nyílik meg, amely a páratlan számokat számolja meg:

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

2. Mellette a teszteset YAML-ben adja meg a bemeneteket és a várt kimeneteket:

   ```yaml
   n: 5
   szamok: [3, 72, 81, 4, 5]
   db: 3
   ```

3. Gépelés közben a háttérben minden kiértékelődik. A teszt füle zöld lesz; ha a `db: 3`-at `db: 2`-re írod át, piros lesz, és megjelenik az érték, amelyet a specifikáció ad.
4. Válts a **Stuki** fülre, és rajzold meg a megszámolás algoritmusát struktogramként.
5. Nyomd meg a **Run tests** gombot: a struktogram lefut ugyanazon a teszteseten, és saját zöld jelet kap.

## Specifikációk

A specifikáció kétféle blokkstílusban írható, `Be:/Sa:/Ki:/Fv:/Ef:/Uf:` vagy `In:/Aux:/Out:/Fn:/Pre:/Post:` alakban, mellette YAML-tesztesetekkel.
Ha kiveszed a pipát az **Auto** elől, csak az **Evaluate** gomb (Ctrl+Enter) megnyomására értékel ki.

### Ugyanazok az ítéletek, mint az online szerkesztőben

A speci-local ugyanazokat a specifikációkat fogadja el és utasítja el, mint az online szerkesztő, és minden tesztesetet ugyanúgy ítél meg. Ezt az online szerkesztő saját kiértékelőjével ellenőriztük 816 eseten.

| | Támogatott |
| --- | --- |
| Típusok | elemi halmazok, intervallumok, tömbök és mátrixok (kezdőértékkel és tetszőleges indextartománnyal), rekordok, típusálnevek, indextartományos tömbliterálok (`[0..:1, 2]`) |
| Függvények | `Fn:`/`Fv:`/`Def:` fejlécenként egy, elöl a szignatúrával |
| Logika | ∀/∃ és szavas alakjaik (`forall`/`minden`, `exists`/`létezik`, `in`/`eleme`), `true`/`igaz`, `false`/`hamis`, `->`, esetszétválasztás `{a, ha c; b egyébként}` |
| Aritmetika | `\|`, `∤`, `<>`, `mod`/`%`, `div`, `n!`, `abs`, `sqrt`, `round`/`kerekít`, `length`/`hossz`, `from`/`tól`, `to`/`ig` |
| Tesztadatok | YAML; egy tömb vagy rekord több sorba is írható, és a tesztesetszerkesztőben az Enter úgy húz be, mint egy JSON-szerkesztő |
| Tételek | `SUM`, `COUNT`, `MAX`, `MIN`, `EXISTS`, `FORALL`, `SELECT`, `SEARCH`, `COPY`, `MIS`, `CONDMAX`, `CONDMIN` és a többi nevük (`SZUMMA`, `MEGSZÁMOL`, `ELDÖNT`, `MIND`, `KERES`, `KIVÁLASZT`, `MÁSOL`/`MAP`, `KIVÁLOGAT`/`FILTER`, `MAXHA`, `MINHA`, …) |

Azt is elutasítja, amit az online szerkesztő elutasít: `→`, `∧`/`∨`, `ABS`, `element`, `SELECT(i≥…)`, több függvény egy blokkban, kimenetek az előfeltételben, az utófeltételből hiányzó deklarált változók, és az olyan tesztesetek, amelyekből kimarad egy érték, vagy amelyek újra megadnak egy konstanst.

### Miért bukik el egy teszt?

- Ha egy teszteset elbukik, az alkalmazás kiszámolja és megmutatja, milyen értékeket adna a specifikáció.
- Ha a kiértékelés hibát talál, annak sora színezve jelenik meg a szerkesztőben. A hibakártyán a helyre kattintva oda ugrik.
- A **Stepper** (az Evaluation panelen választható) lépésenként végigvezet a kiválasztott teszteseten:
  - a formula látszik, benne kiemelve az épp kiértékelt rész, mellette az értéke és az aktuális ciklusváltozók;
  - `↓`/`↑` jelzi, ha belépsz egy saját `Fv:` függvénybe vagy kilépsz belőle, és a függvényen belül annak törzse lesz a mutatott formula;
  - léptetni a ← / → billentyűkkel vagy a gombokkal és a csúszkával lehet. A *Step list* egyszerre mutat minden lépést; egyre kattintva oda ugrik.

### A szerkesztő

- Szintaxiskiemelés, sorszámok és környezetfüggő kiegészítés: blokkfejlécek a sor elején, halmaznevek az `∈` után, valamint a deklarált változók, típusok és függvények.
- A tételek és a kvantorok sablonként szúródnak be, például a `SUM`-ból `SUM(i=1..n, |)` lesz. A ↑↓ választ, a Tab vagy az Enter beszúr, az Esc bezár.
- Különleges karakterek: a szimbólumgombok, az **Alt+1 … Alt+6** (macOS-en **⌥1 … ⌥6**) az ∈ ∀ ∃ ≠ ≤ ≥ jelekhez, vagy írd be, hogy `forall`, `exists`, `leq`, és a kiegészítés szimbólummá alakítja.
- Világos, sötét vagy a rendszert követő téma, és állítható betűméret.

## Stuki (struktogramok)

A két szerkesztő között az eszköztár **Specification** / **Stuki** füleivel lehet váltani. Mindkét fülnek saját fájlja van, és az Új, Megnyitás, Mentés, Másolás és Beillesztés (és a billentyűparancsaik) mindig az aktuális fülre vonatkoznak.

### Szerkesztés

- **Elements**: utasítás, elágazás, elöltesztelős ciklus, hátultesztelős ciklus vagy többirányú elágazás húzható rá egy struktogramra.
  - Egy elem felső vagy alsó felére ejtve elé vagy mögé kerül, egy üres `-` utasításra ejtve pedig a helyére.
  - Ha egy ciklus vagy elágazás mögé szeretnéd tenni, ejtsd az elem alsó szélére (egy vonal jelzi a helyét), vagy közvetlenül a struktogram alá, ha az az utolsó elem.
  - Egy elemre kattintva kijelölöd; a ↑ / ↓ gombok a kijelölt elem fölé vagy alá szúrnak be.
- A szövegek helyben szerkeszthetők. Egy elemet az üres részénél vagy a sarkában lévő fogantyúnál fogva lehet mozgatni. Törölni a piros ✕-szel, a Delete billentyűvel, vagy a struktogramokon kívülre ejtve lehet.
- Egy többirányú elágazás fölé víve az egeret megjelenik a **+C** / **+D** (új eset / az egyébként-ág).
- Egy struktogram fölé víve az egeret megjelenik a **✕ F V R**: törlés, a függvényfej, a lokális deklarációk és a visszatérési érték megjelenítése. A zöld **+** új struktogramot ad hozzá.
- **Visszavonás / újra** a gombokkal vagy a Ctrl+Z / Ctrl+Y billentyűkkel.

### Run tests

A **Run tests** lefuttatja a struktogramokat a Specification fül tesztesetein, és mindegyikhez zöld vagy piros jelet mutat. Minden változás után újra lefut: a struktogramok, a specifikáció vagy a tesztesetek változása után is.

Egy teszteset akkor sikeres, ha a struktogram által a teszt bemeneteiből kiszámolt kimenetek teljesítik az utófeltételt. Egy jel fölé víve az egeret látszik az üzenet és a kiszámolt kimenetek. Egy piros jelre kattintva kijelölődik az elem, ahol a futás megállt.

Az elemek szövegét a specifikációk nyelvén olvassa:

| Szöveg | Jelentés |
| --- | --- |
| `x:=…`, `t[i]:=…`, `r.mező:=…`, `(a, b):=…` egy utasításban | értékadás |
| `i=1..n` egy elöltesztelős ciklus tetején | számlálós ciklus |
| bármi más egy elágazásban, ciklusban vagy esetben | feltétel (`i≤n és nem van`) |
| `-` vagy üres utasítás | semmi |
| `f(i:Egész):Logikai` alakú nevű struktogram | függvény: `f(…)` alakban hívható, és a visszatérési értékét adja |

Az első olyan struktogram, amely nem függvény, a program. A specifikáció azon függvényei, amelyekhez nincs struktogram, a definíciójuk alapján értékelődnek ki.
A futás üzenettel megáll, ha egy szöveg nem olvasható lépésként, egy változót azelőtt használnak, hogy értéke lenne, egy index kívül esik a tartományon, egy többirányú elágazás egyik esete sem teljesül, vagy a program 2 millió lépés után is fut még.

### Képek és színek

- **Másoláskor kép keletkezik**, amely beilleszthető egy feladatba, dokumentumba vagy csetbe: az eszköztár másolás gombja, a **Copy image** vagy a Ctrl+C (szövegmezőn kívül) PNG-képként másolja a kijelölt struktogramot (ha nincs kijelölés, az elsőt). Egy adott struktogram másolásához használd a fölötte lévő sáv kép gombját, vagy kattints rá jobb gombbal. Az **Export PNG** fájlba menti a képet.
- A képek mindig világosak (fehér háttér, fekete vonalak), sötét módban is. Ha a beállításokban kikapcsolod a *Light pictures* opciót, azt kapod, ami a képernyőn látszik.
- **Színek**: szövegszín és szöveghátter a szöveg kijelölt részére (vagy az egészére), elemháttér és struktogramháttér, a projekttel együtt mentett, szerkeszthető palettákról. A **Colorize** fajtájuk szerint színezi az elemeket.
- A csúszkák gomb alatt van a betűtípus és a webalkalmazás haladó beállításai: vízszintes / függőleges távolságok (összekapcsolva vagy külön), illesztés, blokkmagasság, térköz, keretvastagság és jobb oldali margó. Nagyítani a csúszkával, Ctrl+görgővel vagy csippentéssel lehet.

## Fájlok és megosztás

Fájlt a Ctrl+O-val, az ablakra húzva, vagy a parancssorból (`speci-local fajl.sp`) lehet megnyitni. Kilépéskor a munkamenet automatikusan mentődik.

### `.sp`: specifikáció a teszteseteivel

A `.sp` fájl egyszerű szöveg: a specifikáció, amelyben minden teszteset megjegyzésként szerepel.

```text
// title: Páratlan számok
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

Mivel a tesztesetek megjegyzések, az egész fájl továbbra is érvényes specifikáció. Változtatás nélkül beilleszthető az ELTE szerkesztőjébe.

### `.st`: struktogramok

A `.st` fájl a webalkalmazás saját állapota JSON-ban (az általa tárolt `nodes` és `structograms`, valamint a színpaletták), így ugyanazok az adatok mozognak a kettő között. A színes szöveg a webalkalmazás által használt HTML-ként marad meg (`<span style="color: rgb(255, 0, 0);">…</span>`).

### Egy sor az egész projektre

- A Specification fülön a másolás gomb (Ctrl+Shift+C) a specifikációt és az összes tesztesetet egyetlen tömörített sorként, `SPECI1:…` alakban teszi a vágólapra. Csetben könnyű elküldeni.
- A Stuki fülön a **Copy as text**, a Ctrl+Shift+C vagy a Shift+kattintás a másolás gombra egy `STUKI1:…` sort másol.
- A beillesztés (Ctrl+Shift+V) mindkét sort beolvassa, és a megfelelő fülön nyitja meg. Elfogad sima specifikációszöveget, `.st` fájl szövegét és a webalkalmazás megosztási linkjeit is (`…/stuki/v1/?data=…`).
- Ha a rendszer vágólapja nem olvasható, megnyílik egy mező, ahová Ctrl+V-vel be lehet illeszteni.

## Billentyűparancsok

macOS-en a Ctrl helyett ⌘, az Alt helyett ⌥ használandó.

| Billentyű | Művelet |
| --- | --- |
| Ctrl+N / Ctrl+O | Új / megnyitás, az aktuális fülön |
| Ctrl+S / Ctrl+Shift+S | Mentés / mentés másként |
| Ctrl+Shift+C / Ctrl+Shift+V | Az egész projekt másolása / beillesztése egy sorként |
| Ctrl+Enter | A specifikáció kiértékelése |
| Alt+1 … Alt+6 | ∈ ∀ ∃ ≠ ≤ ≥ beszúrása |
| ← / → | Stepper: előző / következő lépés |
| Ctrl+Z / Ctrl+Y | Stuki: visszavonás / újra |
| Ctrl+C | Stuki, szövegmezőn kívül: a struktogram másolása képként |
| Delete | Stuki: a kijelölt elem törlése |
| Ctrl+görgő | Stuki: nagyítás |

## Telefonok és Android

Keskeny képernyőn (telefonon, vagy keskeny ablakban asztali gépen) egyszerre egy panel látszik, és a **Spec / Tests / Results** fülek váltanak közöttük. A beállítások menüben van a *Save as…* és a téma.

- Androidon nincsenek fájlválasztó ablakok. Az **Open** és a **Save** az alkalmazás mappájában lévő fájlok listáját mutatja (`Android/data/io.github.fcsab.specilocal/files/specifications`, amelyet a számítógép USB-n keresztül lát).
- Projektet eszközök között a projekt másolása / beillesztése gombokkal lehet átvinni. Ezek az Android vágólapjával működnek, így a `SPECI1:…` sor bármilyen csetalkalmazáson elküldhető.
- A képernyő-billentyűzet sima betűket, számjegyeket és írásjeleket ír. Az ∈ ∀ ∃ ≠ ≤ ≥ jelekhez használd a szimbólumgombokat, vagy írd be, hogy `forall`, `exists`, `leq`. A szimbólumgombok melletti beillesztés gomb a kurzorhoz illeszti a vágólap tartalmát.
- Egy struktogram másolása a képet az alkalmazás mappájába menti, mert az Android vágólapja nem tud képet tárolni az ilyen alkalmazásokból.

## Fordítás forráskódból

Egy [Rust eszközlánc](https://rustup.rs) kell hozzá (2024-es kiadás).

```sh
cargo run --release
```

Debianon vagy Ubuntun előbb telepítsd a fordításhoz szükséges csomagokat:

```sh
sudo apt-get install libxkbcommon-dev libgtk-3-dev
```

macOS-en a `scripts/bundle-macos.sh` elkészíti a `speci-local.app`-ot (univerzális bináris Apple Siliconra és Intelre), és becsomagolja.

### Nix

```sh
nix run github:FCsab/speci-local            # futtatás telepítés nélkül
nix profile install github:FCsab/speci-local
nix develop                                 # shell cargo, clippy, rustfmt, rust-analyzer eszközökkel
```

Vagy add hozzá az `inputs.speci-local.url = "github:FCsab/speci-local";` sort egy NixOS / Home Manager flake-hez, és használd a `speci-local.packages.${system}.default` csomagot.
A flake a verziót a `Cargo.toml`-ból, a crate-ek hash-eit a `Cargo.lock`-ból olvassa, így egyik változásakor sem kell módosítani; a `flake.lock`-ot (nixpkgs) egy heti pull request frissíti.

### Android

Az androidos alkalmazás az `android/` crate, egy vékony burok ugyanazon könyvtár körül, amely a [cargo-apk](https://github.com/rust-mobile/cargo-apk) eszközzel készül:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-apk
export ANDROID_HOME=~/Android/Sdk ANDROID_NDK_ROOT=~/Android/Sdk/ndk/<verzió>
cargo apk run -p speci-local-android      # debug változat, a csatlakoztatott telefonra telepítve
```

A `--target x86_64-linux-android` kapcsolóval csak az emulátorra készül. Az alkalmazás a logcatbe naplóz (`adb logcat -s speci-local`).
A `cargo apk build -p speci-local-android --release` aláírókulcsot igényel: állítsd be a `CARGO_APK_RELEASE_KEYSTORE` (egy `.jks` fájl) és a `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` változót.

## Fejlesztés

```sh
cargo test                                   # egység-, felület- és integrációs tesztek
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

A CI minden pushnál lefuttatja mindhármat, megépíti az asztali alkalmazást Windowsra, Linuxra és macOS-re, és készít egy debug APK-t.

| Útvonal | Tartalma |
| --- | --- |
| `src/parser/` | a specifikációk lexere, blokkokra bontója, nyelvtana és statikus ellenőrzései |
| `src/eval/` | a kiértékelő: értékek, műveletek, tételek, függvényhívások, a Stepper nyomkövetése |
| `src/data.rs`, `src/runner.rs` | a YAML-tesztesetek beolvasása, futtatása és az ítélet magyarázata |
| `src/file.rs`, `src/compact.rs` | `.sp` fájlok és a `SPECI1:` sor |
| `src/editor/` | a kódszerkesztő: kiemelés, kiegészítés és Vim-billentyűk (`vim/`) |
| `src/app/` | az ablak: eszköztár, panelek, eredmények, Stepper, párbeszédablakok, háttérben futó kiértékelés |
| `src/stuki/` | a struktogramszerkesztő: modell, elrendezés, vászon, színek, PNG és `.st` fájlok |
| `src/stuki/run/` | struktogramok futtatása teszteseteken: fordító és gép |
| `src/platform.rs` | ami az asztali rendszerek és az Android között eltér |
| `android/` | az androidos burok crate |
| `tests/` | esettanulmányok, határesetek, korlátok, és egy fuzzer, amely érvényes dokumentumokat ront el |

Hogyan ellenőrzik egymást a részek:

- A `tests/cases.rs` az ELTE kézikönyvének esettanulmányait tartalmazza.
- A `tests/fuzz.rs` érvényes dokumentumokat ront el több ezer apró módon. A parsernek, a kiértékelőnek és a struktogramfuttatónak mindegyikre ítélettel vagy üzenettel kell válaszolnia, soha nem pánikkal.
- A felülettesztek (`src/app/tests.rs`) a valódi ablakot vezérlik az [egui_kittest](https://crates.io/crates/egui_kittest) segítségével.

A `SKILL.md` a specifikációs nyelv kézikönyve úgy, ahogy az online szerkesztő meghatározza; AI-asszisztensek skilljeként is használható.

## Kiadás

1. Emeld meg a `version` értékét a `Cargo.toml`-ban. A CI magától soha nem ad ki semmit.
2. Nyomd meg a *Run workflow* gombot a Release workflow-n, vagy pushold a megfelelő taget (`git tag v0.5.0 && git push origin v0.5.0`).

A workflow minden platformra megépíti az alkalmazást, lefuttatja a teszteket, és a fájlokat a kiadáshoz csatolja.

Helyben frissülő APK-khoz add hozzá az `ANDROID_KEYSTORE` (egy `.jks` fájl base64 alakja) és az `ANDROID_KEYSTORE_PASSWORD` titkot a repóhoz. Ezek nélkül minden kiadást egy új, eldobható kulcs ír alá, így új verzió telepítése előtt a régit el kell távolítani.

## Ami nem került bele

- A Specifikációszerkesztőből: az előzmények panel és a megosztás állandó linkkel.
- A Stukiból: az INF-bejelentkezés, a mentés az egyetemi szerverre, és a beépítetten kívüli betűtípusok (fix vagy arányos szélességű).

## Köszönet

- Az eredeti webalkalmazások, a Specifikációszerkesztő és a Stuki, az Eötvös Loránd Tudományegyetem Informatikai Karán (ELTE IK) készültek. A speci-local független offline újraírás, és nem áll kapcsolatban az egyetemmel.
- A felület a 0xProto Nerd Font betűtípust használja, az ∈ ∀ ∃ jelekhez a Noto Sans Math tartalék betűtípussal. Mindkettő be van ágyazva, licenceik az `assets/` mappában vannak.
