---
name: specifikacio
description: "Write, debug, and format specifications for the ELTE Specification Editor (progalap.elte.hu/specifikacio) and its offline twin speci-local, using the Hungarian block style (Be:/Sa:/Ki:/Fv:/Ef:/Uf:) or the English one (In:/Aux:/Out:/Fn:/Pre:/Post:) — programming theorems (SUM, COUNT/DARAB, MAX, MIN, EXISTS, FORALL, SELECT, SEARCH/KERES, COPY, MIS, CONDMAX/FELTMAX, CONDMIN), ∀/∃ conditions, functions, and YAML test cases. Use when the user mentions specifikáció, Be:/Ki:/Ef:/Uf:, progalap.elte.hu, speci-local, .sp files, programming theorems (programozási tételek), or ELTE PROGRAMMING (IP-18fPROGEG) tasks."
---

# The ELTE Specification Language — Reference Manual

The **Specification Editor** (*Specifikáció szerkesztő*, [progalap.elte.hu/specifikacio](https://progalap.elte.hu/specifikacio/)) is the formal-specification tool of the ELTE IK course **PROGRAMMING** (*Programozás alapjai*, `IP-18fPROGEG`). A specification declares the state space (input, auxiliary and output variables), a precondition and a postcondition; test cases give concrete values for every variable, and the editor evaluates the conditions on them and marks each test green or red.

**speci-local** is an offline reimplementation of that editor (plus the Stuki structogram editor) in one native app. It accepts and rejects the same specifications and judges test cases the same way as the online editor (checked against the online evaluator on 816 cases). Everything in this manual is checked against speci-local's parser and evaluator; where the online editor's own error text is known, it is quoted.

## Contents

1. [Quick rules and the default answer format](#1-quick-rules-and-the-default-answer-format)
2. [The tools](#2-the-tools)
3. [Document structure](#3-document-structure)
4. [Types and declarations](#4-types-and-declarations)
5. [Expressions and operators](#5-expressions-and-operators)
6. [Functions (`Fv:` / `Fn:`)](#6-functions-fv--fn)
7. [Preconditions (`Ef:` / `Pre:`)](#7-preconditions-ef--pre)
8. [Postconditions and programming theorems](#8-postconditions-and-programming-theorems)
9. [Test cases](#9-test-cases)
10. [How a test case is judged](#10-how-a-test-case-is-judged)
11. [Case studies](#11-case-studies)
12. [Pitfalls and error messages](#12-pitfalls-and-error-messages)
13. [Grammar](#13-grammar)
14. [Checklist before answering](#14-checklist-before-answering)

---

## 1. Quick rules and the default answer format

When asked to write or fix a specification, answer in this layout unless the user's file or request says otherwise:

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

```yaml
n: 5
szamok: [3, 72, 81, 4, 5]
db: 3
```

The rules that matter most:

1. **Blocks and order.** Declarations first (`Be:`, `Sa:`, `Ki:` in any order among themselves), then functions (`Fv:`, one function per header), then `Ef:`, then `Uf:`. `Be:`, `Ki:`, `Ef:` and `Uf:` are compulsory; write `Ef: -` when there is no precondition.
2. **Headers are exact and case-sensitive:** `Be: Sa: Ki: Fv: Ef: Uf:` or `In: Aux: Out: Fn: Pre: Post:` (`Def:` is another function header). After the colon comes a space (content on the same line) or a line break. `Segéd:`, `in:`, `Pre :` and `Ef:-` are all errors.
3. **Hungarian by default:** Hungarian headers *and* Hungarian identifiers (`szam`, `ar`, `tav`, `db`, `van`, `ind`, `sorszam`, `ertek`) unless the user writes in English or edits an English file. Never silently translate a working file from one style to the other.
4. **Every declared variable must appear in the postcondition**: inputs, auxiliary variables, outputs and constants. Appearing only in `Ef:` does not count. Add `and x=x` for one that the formula really doesn't need.
5. **Every declared variable needs a value in every test case**, auxiliary ones too (`van`, `ind`), except constants declared with an initial value: those must *not* appear in the test data.
6. **ASCII where the glyph is refused:** implication is `->` (the glyph `→` is rejected everywhere), function arrows are `->`, and inside `SELECT` only `>=` works (`SELECT(i>=1, …)`). `∈ ∀ ∃ ≤ ≥ ≠` are fine elsewhere; `<= >= !=` work too.
7. **Reserved words are not names:** `exists`, `forall`, `in`, `and`, `or`, `not`, `mod`, `div`, `true`, `false` and the Hungarian `létezik`, `minden`, `eleme`, `és`, `vagy`, `nem`, `igaz`, `hamis`. Call a search flag `van` or `talalt`, never `exists`.
8. **Theorem macros are uppercase** (`SUM`, `MAX`, `KERES`, `FELTMAX`, …), so lowercase variables like `max`, `sum`, `db` never clash with them. Built-in functions are lowercase (`abs`, not `ABS`).
9. **An output that can be `-1` is typed `Z`**, never `N`.
10. **Parenthesize quantifier bodies:** `∀i∈[1..n]:(t[i]>0)`. Without the parentheses only `t[i]` is the body.

---

## 2. The tools

### 2.1. The online editor (progalap.elte.hu/specifikacio)

Requires an ELTE (INF / Caesar) login. Panes: Specification, Data (test cases, one tab per case, `+`/`-` to add or remove), Visualization (Stepper and Step list), and a history sidebar. A symbol palette inserts `∈ ∀ ∃ ≠ ≤ ≥`. In the Data pane, `Shift+Space` then `Tab` scaffolds the declared variable names. Evaluating colours each test tab green or red. Saving gives a permalink; share links carry the whole project as `…/specifikacio/v1/?data=…`.

### 2.2. speci-local (offline)

A native app for Windows, Linux, macOS and Android. It needs no login and no network.

- **Evaluation** runs in the background as you type. Untick **Auto** to evaluate only on **Evaluate** (Ctrl+Enter).
- **When a test fails**, it shows what the specification would give: "`db`: the test case says 2, the specification gives 3". An error tints its line, and each error card links to the documentation at <https://fcsab.web.elte.hu/en/>.
- **Scaffold** inserts a `name: ` line for every variable missing from the test case. It skips constants, and pre-fills `[]` for arrays and `{field: }` for records.
- **Stepper** (Evaluation pane) replays one test case reduction by reduction. The part being evaluated is highlighted, the loop variables are listed, and `↓`/`↑` mark entering and leaving your functions. Step with ←/→. *Step list* shows every step.
- **Editor:** completion for headers, set names after `∈`, declared names, and pattern snippets (`SUM` → `SUM(i=1..n, |)`). Alt+1…Alt+6 insert `∈ ∀ ∃ ≠ ≤ ≥`. Typing `forall`, `exists` or `leq` and completing turns them into symbols. `<=`, `>=` and `!=` typed by hand become `≤ ≥ ≠`, so type SELECT's `>=` through the `SELECT` completion snippet.
- **`.sp` files** are plain specification text with each test case stored as a comment, so the whole file is still a valid specification and can be pasted into the online editor as-is:

  ```text
  // title: Páratlanok
  Be:
  n∈N,
  szamok∈N[1..n]

  Ki:
  db∈N

  Ef:
  -

  Uf:
  db=DARAB(i=1..n, szamok[i] mod 2 = 1)

  /*--- test ---
  n: 5
  szamok: [3, 72, 81, 4, 5]
  db: 3
  ---*/
  ```

- **Sharing:** Ctrl+Shift+C copies the whole project as one `SPECI1:…` line, and Ctrl+Shift+V pastes it back. Paste also accepts plain specification text and the web editors' `?data=` links. *Open in web editor* and *Copy web editor link* hand the project to the online editor.
- **Stuki tab:** draw structograms and **Run tests** on the same test cases. A test passes when the outputs computed by the structogram satisfy the postcondition.

---

## 3. Document structure

### 3.1. Blocks

| Block | Hungarian | English | Compulsory | Holds |
|:--|:--|:--|:--:|:--|
| Input | `Be:` | `In:` | yes | input variables, type aliases |
| Auxiliary | `Sa:` | `Aux:` | no | helper variables (search flags, indexes) |
| Output | `Ki:` | `Out:` | yes | result variables |
| Function | `Fv:` | `Fn:` (also `Def:`) | no | **one** function per header; repeat the header for more |
| Precondition | `Ef:` | `Pre:` | yes | condition on the input |
| Postcondition | `Uf:` | `Post:` | yes | condition linking input and output |

- **Order:** all declaration blocks (`Be`/`Sa`/`Ki`, any order among themselves) → `Fv` blocks → `Ef` → `Uf`. Breaking it is an error: "`Ki:` is in the wrong place … Declarations (In/Aux/Out) come first, then Fn, then Pre, then Post." The online editor says: `Expected ",", "Aux:", "Ki:", "Out:", "Sa:", new line, or white space but "S" found.`
- **Each block once**, except the function block. A second `Be:` is an error, so merge the two with commas.
- **Hungarian and English headers may be mixed**, but don't do it: keep one style per file.
- **A header starts a line** (indentation allowed): a word, the colon, then a space or a line break. Any other line whose first word is capitalised and followed by `:` is read as a misspelt header. Translations are rejected with a pointer to the real header (`Segéd:` → `Sa:`, `Bemenet:` → `Be:`, `Precondition:` → `Pre:`), and wrong case is rejected too (`in:` → `In:`).
- **Only comments may come before the first block.**

### 3.2. Two layouts

**Layout A**, content on the header's line:

```text
In: n∈N, nums∈Z[1..n]
Out: cnt∈N
Pre: -
Post: cnt=COUNT(i=1..n, nums[i] mod 2 = 1)
```

```yaml
n: 5
nums: [3, 72, 81, 4, 5]
cnt: 3
```

**Layout B**, header alone on its line, one declaration per line, a comma after each declaration except the last, and a blank line between blocks (the preferred Hungarian layout):

```text
Be:
n∈N,
k∈N,
ar∈N[1..n]

Sa:
van∈L,
ind∈N

Ki:
sorszam∈Z

Fv:
kisebb:N->L,
kisebb(i)=ar[i]<k

Ef:
1<=n<=100 és 1<=k<=10000

Uf:
(van,ind)=KERES(i=1..n, kisebb(i)) és
(van -> sorszam=ind) és
(nem van -> sorszam=-1)
```

```yaml
n: 6
k: 2000
ar: [2500, 5300, 1900, 2400, 8800, 5300]
van: true
ind: 3
sorszam: 3
```

A block's content runs until the next header, so formulas may continue over several lines and blank lines are harmless. A **trailing comma** after the last declaration of a block is an error ("Trailing comma").

### 3.3. Comments

`// …` runs to the end of the line, and `/* … */` may span lines. Both may follow real content. Comment markers inside a text literal (`"//"`) are text.

```text
Be: szam∈N
Ki: osztok∈N
Ef: szam>=1
Uf: osztok=DARAB(i=1..szam, i|szam)    // szam mod i = 0
/* (db,y)=KIVÁLOGAT(i=e..u, T(i), f(i))
   e..u ~ 1..szam,  T(i) ~ i|szam */
```

```yaml
szam: 12
osztok: 6
```

### 3.4. Names

- A name starts with a letter or `_`, followed by letters, digits or `_`. Accented letters are allowed (`szám`), but plain ASCII is the safer habit. Names are case-sensitive.
- **Variables:** lowercase by convention (`n`, `szamok`, `maxTav`, `x1`). **Types:** start with an uppercase letter. This is enforced for aliases: `Varos=(…)` is an alias, `varos=…` is an error.
- **Reserved words** (operators, so never names): `and és or vagy not nem mod div in eleme exists létezik forall minden true igaz false hamis`. The online editor reports `Aux: exists∈L` as `incomplite type for exists: undefined is not boolean`. speci-local says "`exists` is a reserved word and can't be used as a name".
- Not reserved: `x` (it means "product" only inside types), `element` (it is *not* a synonym of `∈`), `ha`/`if`/`egyébként`/`otherwise` (special only inside `{…}` cases), and every lowercase spelling of a macro (`max`, `sum`, `van`, `db`). A variable may even share a built-in's name (`abs∈Z[1..n]` next to `abs(x)`).

---

## 4. Types and declarations

A declaration is `name∈Type`. `in` and `eleme` are word forms of `∈`, but `element` is not. Declarations are separated by commas. A type alias `Name=Type` may stand anywhere in a declaration block, before or after its use, and aliases may chain.

### 4.1. Elementary sets

| Set | Meaning | Test-data values |
|:--:|:--|:--|
| `N` | natural numbers 0, 1, 2, … | whole numbers ≥ 0 (a negative one is rejected: "declare it as Z") |
| `Z` | integers | whole numbers |
| `R` | reals | numbers (`1.5`, `3`) |
| `L` | logical | `true`/`false` or `igaz`/`hamis` |
| `S` | text | `abc`, `"abc"`, `""` for empty |
| `C`, `K` | character | a single character |

### 4.2. Intervals

`j∈[1..5]` is a whole number between the bounds, and a test value outside them is rejected. Bounds may be expressions of earlier inputs: `[2+3..n*2]`.

### 4.3. Arrays (sequences)

| Declaration | Indexes | Test data |
|:--|:--|:--|
| `x∈Z[1..n]` | 1..n | exactly n elements |
| `x∈Z[0..n-1]`, `x∈Z[-1..1]` | as written | exactly that many |
| `x∈Z[1..]` | from 1, length from the data | any length |
| `x∈Z[0..]` | from 0 | any length |
| `x∈Z[]` | same as `Z[1..]` | any length |
| `x∈S[1..8]=["a","b",…]` | constant with initial value | **must not** appear in test data |

`length(x)`/`hossz(x)`, `from(x)`/`tól(x)` and `to(x)`/`ig(x)` give the length, first index and last index. Indexing outside the range is a runtime error ("index 3 is out of range … its indexes are 1..2").

### 4.4. Matrices

`m∈N[1..n,1..m]` is an n×m matrix, indexed as `m[i,j]`. `Z[,]` means `Z[1..,1..]`. Test data gives a list of rows: `[[1,2],[3,4]]`. speci-local stores `Z[1..n,1..m]` and `Z[1..n][1..m]` the same way (an array of rows), so `m[i,j]` and `m[i][j]` both work. Write the comma form for a rectangular grid. Use `Z[1..n][]` only when the rows may have different lengths. Every row of `Z[1..n,1..m]` must have exactly m elements.

### 4.5. Records and products

```text
Be: n∈N, hallgatok∈Hallgato[1..n], Hallgato=(nev:S x jegy:[1..5])
Ki: db∈N
Ef: -
Uf: db=DARAB(i=1..n, hallgatok[i].jegy=5)
```

```yaml
n: 3
hallgatok:
  - nev: Anna
    jegy: 5
  - nev: Bence
    jegy: 3
  - nev: Csilla
    jegy: 5
db: 2
```

Fields are separated by ` x ` (`*` is also accepted). A comma between fields is an error. A field is read as `r.field`. A record type may also be written inline: `r∈(nev:S x kor:N)`. Fields may be arrays or other records: `Diak=(nev:S x jegyek:[1..5][])`.

### 4.6. Constants

`x∈T=expr` gives the variable its value in the specification. It is still a declared variable: it must appear in `Uf:`, and it must **not** be repeated in the test data. The online editor says `data cannot contain item with the same name: x`. An initialised matrix keeps the declared index ranges: `m∈Z[-1..1,-1..1]=[[2,1,2],[1,0,1],[2,1,1]]` gives `m[-1,-1]=2`.

---

## 5. Expressions and operators

### 5.1. Precedence (loosest first)

| Level | Operators | Notes |
|:--:|:--|:--|
| 1 | `or`, `vagy` | |
| 2 | `and`, `és` | |
| 3 | `->` | implication, right-associative, **binds tighter than `and`/`or`** |
| 4 | `not`, `nem`, `¬` | prefix |
| 5 | `=` `≠` `!=` `<>` `<` `>` `≤` `<=` `≥` `>=`, `\|` (divides), `∤`, `x∈[a..b]` | comparisons chain: `1<=n<=100`, `2≤a≤7` |
| 6 | `+` `-` | `+` also joins texts |
| 7 | `*` `/` `mod` `%` `div` | |
| 8 | unary `-`, `not` | |
| 9 | `t[i]`, `m[i,j]`, `r.field`, `p.1`, `n!` | postfix |

Consequences:

- `van -> r=ind and not van -> r=-1` is two implications joined by `and`, which is the intended reading. Still, parenthesize them for clarity: `(van -> r=ind) and (not van -> r=-1)`.
- `a or b -> c` means `a or (b -> c)`. Parenthesize when mixing.
- `not a=b` means `not (a=b)`.

### 5.2. Logic and comparison

- `true`/`igaz`, `false`/`hamis`. `∧`, `∨` and `→` are **rejected** ("Unexpected character"); write `and`/`és`, `or`/`vagy`, `->`.
- Equality compares by value. Numbers compare across `Z`/`R` (within 1e-9). Texts compare exactly and order lexicographically with `<`. Records compare field by field. **Arrays are equal only if their index ranges match too:** `[1..2]` vs `[0..1]` is an error, not `false`.
- `x∈[a..b]` tests membership in an interval: `Ef: a∈[2..7]`.

### 5.3. Arithmetic

| Operator | Meaning |
|:--|:--|
| `+ - *` | overflow is an error, not a wrap-around |
| `/` | exact quotient: whole if divisible, otherwise real (`3/2 = 1.5`) |
| `div` | whole division rounded down (`-7 div 2 = -4`) |
| `mod`, `%` | remainder, sign follows the dividend (`-7 mod 2 = -1`) |
| `a\|b` | *a divides b* (`i\|n` ⇔ `n mod i = 0`); `a∤b` is its negation |
| `n!` | factorial |

Built-in functions (lowercase only): `abs`, `sqrt` (real result), `round`/`kerekít` (halves round up), `length`/`hossz`, `from`/`tól`, `to`/`ig`. `ABS(x)` is rejected: write `abs(x)`.

### 5.4. Literals, tuples and cases

- Numbers: `42`, `-3`, `2.5`. Texts: `"alma"`, `'alma'` or `„alma”`.
- Array literals: `[1, 2, 3]` (indexed from 1), `[0..:5, 6]` (from 0), `[1..,1..:[1,2],[3,4]]` (a matrix with explicit starts).
- Tuples: `(a, b)`. A part is read as `.1`, `.2`, … (1-based): `(3,5).2 = 5`. An empty slot `(a,,c)` is only allowed on the left side of `=`, to skip a pattern result.
- **Cases** pick a value by condition: the first case whose condition holds gives the value.

  ```text
  {érték1, ha feltétel1; érték2, ha feltétel2; érték3 egyébként}
  {1, if x>0; 0, if x=0; -1 otherwise}
  ```

  The commas before `ha` and `egyébként` are optional. If no case holds and there is no `egyébként`, evaluation stops with "None of the cases … holds".

### 5.5. Quantifiers

```text
∀i∈[1..n]:(t[i]>0)          forall i in [1..n]:(t[i]>0)        minden i eleme [1..n]:(t[i]>0)
∃i∈[1..n]:(t[i] mod 2 = 1)  exists i in [1..n]:(…)             létezik i eleme [1..n]:(…)
```

- The range is always an interval `[lo..hi]`. Over an empty range, `∀` is true and `∃` is false.
- **The body must be in parentheses.** `∀i∈[1..n]:t[i]>0` is parsed as `(∀i∈[1..n]:t[i]) > 0` and fails with "`t[i]` should be true or false".
- Nest them for matrices: `∀i∈[1..n]:(∀j∈[1..m]:(a[i,j]>=0))`.
- The quantified variable is local. It may reuse a name from an enclosing formula, but avoid it for readability.

---

## 6. Functions (`Fv:` / `Fn:`)

```text
Fv: f:Dom->Cod, f(p)=body
```

- **One function per header.** `Fn: f:…, f(…)=…, g:…, g(…)=…` is rejected ("Only one function per `Fn:` header"); start each function on its own `Fv:` line. Consecutive `Fv:` headers are fine.
- **The signature is compulsory** and comes first. `Fv: d(a)=a*2` alone is rejected ("has no signature"). The name in the definition must match the signature.
- **The arrow is ASCII `->`.** Several parameters go in a product domain: `add:Z x Z->Z, add(a,b)=a+b`. Array parameters are written `Z[]`.
- The body is one expression, often a pattern. It sees every declared variable, and parameters shadow them. Functions may call other functions and themselves (recursion depth up to 500).
- **Tuple-valued patterns need the whole tuple in the codomain.** `MAX`/`MIN` give 2 values, `SEARCH` 2, `MIS` 2 and `CONDMAX`/`CONDMIN` 3. `f:N->N, f(i)=MAX(…)` is rejected ("`MAX` gives 2 values (index, value), but the signature of `f` has 1 in its codomain"). Write `f:N->N x N` and use `f(i).2`, or keep one part in the body: `f(i)=MAX(…).2`.

Examples (each used in a case study below):

```text
Fv: maxsor:N->N x N, maxsor(i)=MAX(j=1..m, t[i,j])
Fv: osszeg:N->N, osszeg(i)=SZUMMA(j=i-d..i-1, tr[j])
Fv: mind5felett:N->L, mind5felett(i)=MIND(j=1..4, kave[i,j]>5)
Fv: uj:N->L, uj(i)=MIND(j=1..i-1, futas[i].ido≠futas[j].ido)
Fv: elojel:Z->Z, elojel(x)={1, ha x>0; 0, ha x=0; -1 egyébként}
Fv: fakt:N->N, fakt(i)={1, ha i=0; i*fakt(i-1) egyébként}
```

---

## 7. Preconditions (`Ef:` / `Pre:`)

- **No precondition:** `Ef: -` (an en dash `–` also works). `Ef: true`/`igaz` is equivalent.
- **Ranges:** `1<=n<=100`, `n∈[1..100]`, `n>=1 and n<=100`.
- **Element constraints:** `∀i∈[1..n]:(1<=ar[i] and ar[i]<=10000)`.
- **Existence**, needed before `SELECT`: `∃i∈[1..n]:(t[i] mod 2 = 1)`.
- **Outputs may not appear in the precondition.** The online editor says `Output data x is in the precondition!`; speci-local says "Output `x` is used in the precondition". Auxiliary variables are allowed.
- **Copy every bound from the statement separately.** A bound that is too loose still evaluates green, so this bug never shows on the samples.
- A test case whose input violates the precondition is red ("The precondition (Ef) is false: `1<=k<=10000` does not hold … k = 0").

---

## 8. Postconditions and programming theorems

### 8.1. The All Variables Mandate

Every declared variable (inputs, auxiliary variables, outputs and constants) must occur in the postcondition, either directly or inside a function the postcondition calls. Occurring only in `Ef:` does not count.

- Online editor: `Line 5, column 1: egyeb is missing from the post condition!` (the position is the **declaration**).
- speci-local: "Input `egyeb` is never used in the postcondition", with the hint "append `and egyeb=egyeb`".

The filler `and x=x` satisfies the check. A bound variable such as `n` in `i=1..n` or `∀i∈[1..n]` counts as used.

### 8.2. Pattern table

`i=b..e` is an inclusive loop range. The loop variable is local to the pattern. Macro names are uppercase; every alias below behaves exactly like its main name.

| Theorem | Main name | Other names | Arguments after the range | Result |
|:--|:--|:--|:--|:--|
| Summation (*összegzés*) | `SUM` | `SZUMMA`, `SZUM` | `f(i)` [, `cond`] | number (0 over an empty range) |
| Counting (*megszámolás*) | `COUNT` | `DARAB`, `MEGSZÁMOL` | `cond` | natural number |
| Maximum (*maximumkiválasztás*) | `MAX` | — | `f(i)` | `(ind, value)`, the **first** maximum |
| Minimum | `MIN` | — | `f(i)` | `(ind, value)`, the first minimum |
| Decision (*eldöntés*) | `EXISTS` | `ELDÖNT`, `VAN`, `LÉTEZIK`, `DECISION` | `cond` | `L` |
| Decision, all | `FORALL` | `MIND`, `EVERY` | `cond` | `L` |
| Selection (*kiválasztás*) | `SELECT` | `KIVÁLASZT` | written `SELECT(i>=b, cond)` | index |
| Linear search (*keresés*) | `SEARCH` | `KERES` | `cond` | `(found, ind)` |
| Copy (*másolás*) | `COPY` | `MÁSOL`, `MAP` | `f(i)` | array indexed `b..e` |
| Multiple selection (*kiválogatás*) | `MIS` | `KIVÁLOGAT`, `FILTER` | `cond` [, `f(i)`] | `(count, array from index 1)` |
| Conditional maximum (*feltételes maximumkeresés*) | `CONDMAX` | `FELTMAX`, `MAXHA`, `MAXIF` | `f(i), cond` | `(found, ind, value)` |
| Conditional minimum | `CONDMIN` | `FELTMIN`, `MINHA`, `MINIF` | `f(i), cond` | `(found, ind, value)` |

A wrong argument count is an error, for example "COUNT got 2 argument(s) after the range, but needs 1". A lowercase macro name (`sum(…)`) is "Unknown function `sum`" with the hint "Write `SUM(…)`".

### 8.3. Using the results

- **Scalar results** are bound or compared with `=`: `db=DARAB(…)`, `s=SUM(…)`, `van=EXISTS(…)`.
- **Tuple results** are unpacked: `(ind,ertek)=MAX(…)`, `(van,ind)=KERES(…)`, `(db,y)=KIVÁLOGAT(…)`, `(van,ind,ertek)=FELTMAX(…)`.
- **Skipping a slot:** `(,ertek)=MAX(…)` keeps the value, `(ind,)=MAX(…)` keeps the index, `(van,,ertek)=FELTMAX(…)` skips the index. The number of slots must match the pattern ("`(a,b)` has 2 slots, but … gives 3 values").
- **Reading one part:** `MAX(…).2`, `KERES(…).1`. MAX and MIN parts also have names: `.maxind`/`.maxval` and `.minind`/`.minval`. speci-local also knows `.found`/`.ind` (SEARCH), `.cnt`/`.items` (MIS) and `.found`/`.maxind`/`.maxval` (CONDMAX). The positional form works everywhere.

### 8.4. Each pattern

**SUM.** `s=SUM(i=1..n, t[i])`. The optional third argument filters the terms: `SUM(i=1..n, i, i|n)` is the sum of the divisors of n. Put **every** restriction from the statement into the filter: for "the odd divisors", write `SUM(i=1..n, i, i mod 2 = 1 and i|n)`, not just `i mod 2 = 1`.

**COUNT / DARAB.** `db=DARAB(i=1..n, t[i]>0)`. It takes exactly one condition; join several conditions with `and`.

**MAX / MIN.** `(ind,ertek)=MAX(i=1..n, t[i])`. Ties go to the first index. Numbers and texts can be compared. **An empty range is an error** ("MAX over an empty range (1..0)"), so require `n>0` in `Ef:` or use `CONDMAX`/`FELTMAX`.

**EXISTS / FORALL.** `van=EXISTS(i=1..n, t[i]<0)` is the macro form of `van=∃i∈[1..n]:(t[i]<0)`.

**SELECT / KIVÁLASZT.** `ind=SELECT(i>=1, t[i] mod 2 = 1)`. There is no upper bound: the pattern assumes a match exists, so the precondition must prove it (`∃i∈[1..n]:(…)`). Because nothing else in the formula mentions `n`, add `and n=n`. **Write `>=`, not `≥`**: inside SELECT the online editor accepts only `>=` (speci-local: "Expected `>=` but found `≥`"). If no match exists, evaluation stops with "SELECT found no matching element".

**SEARCH / KERES.** `(van,ind)=KERES(i=1..n, t[i]<0)` gives the first match. When nothing matches, `van` is false and **`ind` has no defined value**:

- In the test case, `ind` still needs a value, and **any valid value passes** (`0`, `n+1`, …). The editor does not compute it.
- Ordering comparisons with the undefined index are errors ("`ind` has no defined value"), and so is indexing with it. Guard every use with `van -> …`.

**Reverse search** (last match): negate the range and the index, `(van,-ind)=KERES(i=-n..-1, t[-i]>0)`.

**COPY / MÁSOL.** `y=COPY(i=1..n, abs(t[i]))`. The result is indexed `b..e`, so the output's declared start must match (`y∈Z[1..n]` for `i=1..n`).

**MIS / KIVÁLOGAT.** `(db,y)=KIVÁLOGAT(i=1..n, t[i]>0)` collects the **indexes** `i` that match. The three-argument form `(db,y)=KIVÁLOGAT(i=1..n, t[i]>0, t[i])` collects `f(i)`: element values, record fields, or anything else. `y` is indexed from 1. Declare it `y∈N[1..db]` (or `y∈Z[]`).

**CONDMAX / FELTMAX.** `(van,ind,ertek)=FELTMAX(i=1..n, f(i), cond(i))`: note the order, value first and condition second. When nothing qualifies, `van` is false and `ind`/`ertek` are undefined, so any test value passes. Guard their use with `van -> …`.

### 8.5. Implication: outputs that only matter sometimes

```text
(van,ind)=KERES(i=1..n-1, pl[i].y<pl[i+1].y) and
(van -> konyvtar=pl[ind].nev)
```

When `van` is false, the implication is true without looking at `konyvtar`, so the test may give any value for it (`""`). A task that demands a sentinel must state it in the postcondition, because the pattern never produces one:

```text
(van -> sorszam=ind) and (nem van -> sorszam=-1)
```

### 8.6. Relational style (no macros)

A postcondition can be any first-order formula. The lecture notes often use this style, and it needs no auxiliary variables:

```text
Uf: ∃i∈[1..n]:(∀j∈[1..n]:(tav[j]<tav[i] vagy (tav[j]=tav[i] és ar[j]>=ar[i])) és ar[i]=max)
```

Bind outputs **through the witness** (`ar[i]=max`, `sorszam=i`), and never index with a value that may be a sentinel (`ar[sorszam]` when `sorszam=-1` is an out-of-range read).

| Use… | When |
|:--|:--|
| macro style (`MAX`, `KERES`, `KIVÁLOGAT`, …) | the specification should map onto programming theorems and the algorithm (structogram) is derived from them |
| relational style (`∃`/`∀`) | the output is defined by a property, or the task asks for the plain relational model |

### 8.7. Choosing a theorem

- **One number from many:** sum → `SUM`; how many → `DARAB`; largest/smallest → `MAX`/`MIN`; largest among those that satisfy a condition → `FELTMAX`/`FELTMIN`.
- **Yes/no:** is there one → `EXISTS` or `∃`; are all → `FORALL` or `∀`.
- **Where:** guaranteed to exist (proved in `Ef:`) → `SELECT`; may not exist → `KERES`; the last one → reverse `KERES`.
- **Many results:** the matching indexes or values → `KIVÁLOGAT`; transform every element → `COPY`.
- **Nested** (row maximums, a sum per window, all-of-row): put the inner theorem in an `Fv:` function and call it from the outer one.

---

## 9. Test cases

Each test case is YAML: one `name: value` line per variable.

```yaml
# comments start with #
n: 3
t: [5, -1, 7]           # array: exactly n elements for t∈Z[1..n]
nev: Anna               # text, bare or quoted; "" is the empty text
van: true               # or igaz / hamis
m: [
  [1, 2, 3],
  [4, 5, 6]
]                       # matrix: a list of rows, may span lines
p: {nev: L1, x: 12, y: 24}   # record, inline
lista:                  # array of records, block style (2-space indent)
  - nev: AS
    ido: 37
  - nev: BE
    ido: 31
ures: []                # empty array
```

Rules:

- **Every declared variable** (inputs, auxiliary variables and outputs) needs a line, even when its value doesn't matter, like `ind` after a failed search ("Auxiliary variable `ind` is missing from this test case").
- **Constants with an initial value must not** have a line (`data cannot contain item with the same name: lapok`).
- **No other keys:** an undeclared or misspelt key is an error ("`Ar` is not declared … Did you mean `ar`").
- **Types are checked:** a negative number for `N`, a value outside an interval, a text for `Z`, an unknown or missing record field, or a wrong array length ("`ar` has 2 element(s), but it is declared as N[1..n] and n = 3, so it needs exactly 3").
- Arrays take their first index from the declaration: for `t∈Z[0..]`, `[5, 6]` means `t[0]=5`.
- Use spaces, never tabs. In speci-local, Tab inserts spaces and Enter indents like a JSON editor.

---

## 10. How a test case is judged

1. The test data is read and checked against the declarations (section 9). A data error makes the test red.
2. The precondition is evaluated. If it is false, the test is red ("This test case's input is not allowed by the precondition").
3. The postcondition is evaluated with **all** values from the test case. If it is true, the test is green; otherwise it is red. With every variable given, `=` is a comparison.
4. When a test is red, speci-local re-evaluates the postcondition without the expected outputs to show what the specification gives. Then an output with no value is set by the first equation that defines it (`db=…`, `(van,ind)=KERES(…)`, or a witness inside `∃`), reading left to right. Keep the defining equation before other uses of the output. If even that fails, it reports "no output values satisfy the postcondition for this input. The formula itself is probably wrong."

Runtime errors stop the test: index out of range, division by zero, a value of the wrong kind ("`+` needs numbers"), MAX over an empty range, SELECT without a match, use of an undefined search index, no matching case, recursion that is too deep, or an evaluation over 50 million steps. The error message points at the line and column.

---

## 11. Case studies

All of these are green in speci-local. They come from the course materials, so most use English headers and names. When answering, apply the Hungarian default from section 1 unless the user writes in English.

### Task 1: The Discworld libraries (search in records, implication)

> *The Librarian stores the name and coordinates of each library. Give the first library in the list that is further north (smaller y) than the next one.*

```text
In: n∈N, pl∈Tpl[1..n], Tpl=(name:S x x:N x y:N)
Aux: van∈L, ind∈N
Out: lib∈S
Pre: ∀i∈[1..n]:(pl[i].x≥0 and pl[i].y≥0)
Post: (van,ind)=SEARCH(i=1..n-1, pl[i].y<pl[i+1].y) and
      (van -> lib=pl[ind].name)
```

```yaml
# found: L4 (y=4) is north of L5 (y=47)
n: 5
pl: [
  {name: "L1", x: 12, y: 24},
  {name: "L2", x: 5, y: 15},
  {name: "L3", x: 69, y: 4},
  {name: "L4", x: 12, y: 4},
  {name: "L5", x: 212, y: 47}
]
lib: L4
van: true
ind: 4
```

```yaml
# not found: lib and ind are unconstrained, any value passes
n: 4
pl: [{name: L1, x: 10, y: 50}, {name: L2, x: 20, y: 40}, {name: L3, x: 30, y: 30}, {name: L4, x: 40, y: 20}]
lib: ""
van: false
ind: 0
```

```yaml
# n=1: the range 1..0 is empty
n: 1
pl: [{name: L1, x: 0, y: 10}]
lib: ""
van: false
ind: 1
```

The loop stops at `n-1` because the condition reads `pl[i+1]`.

### Task 2: Yossarian's map (maximum of row maximums)

> *On an n×m grid map, Yossarian recorded how many times he was at each place. Give the coordinates of the first most-visited place.*

```text
In: n∈N, m∈N, map∈N[1..n,1..m]
Out: mpx∈N, mpy∈N
Fn: maxcol:N->N x N, maxcol(i)=MAX(j=1..m, map[i,j])
Pre: n>0 and m>0
Post: (mpx,)=MAX(i=1..n, maxcol(i).maxval) and
      mpy=maxcol(mpx).maxind
```

```yaml
n: 6
m: 4
map: [
  [12, 3, 1, 4],
  [3, 2, 23, 5],
  [4, 2, 2, 1],
  [5, 2, 23, 0],
  [0, 2, 4, 1],
  [0, 0, 0, 1]
]
mpx: 2
mpy: 3
```

```yaml
# all equal: the first cell wins
n: 2
m: 2
map: [[7, 7], [7, 7]]
mpx: 1
mpy: 1
```

`maxcol` returns the whole `(index, value)` tuple, so its codomain is `N x N`. The row maximums are 12, 23, 4, 23, 4 and 1; the first 23 is in row 2, and that row's maximum is in column 3.

### Task 3: Trurl's robots (MIS with a window sum)

> *For n days Trurl recorded how many robots he built. List the days on which he built more than the total of the previous d days.*

```text
In: n∈N, d∈N, tr∈N[1..n]
Out: cnt∈N, days∈N[1..cnt]
Fn: total:N->N, total(i)=SUM(j=i-d..i-1, tr[j])
Pre: n>d>0
Post: (cnt,days)=MIS(i=d+1..n, total(i)<tr[i], i)
```

```yaml
n: 10
d: 3
tr: [2, 4, 5, 1, 13, 1, 1, 1, 4, 5]
cnt: 2
days: [5, 9]
```

```yaml
n: 5
d: 2
tr: [100, 50, 20, 10, 5]
cnt: 0
days: []
```

The loop starts at `d+1` so that the window `i-d..i-1` stays inside the array. The two-argument form `MIS(i=d+1..n, total(i)<tr[i])` would collect the same indexes.

### Task 4: Catbert's coffee breaks (search with a nested FORALL)

> *Four coffee breaks a day. Find a day on which more than 5 litres were drunk at every break.*

```text
In: n∈N, coffe∈N[1..n,1..4]
Aux: van∈L
Out: ind∈N
Fn: morethan5:N->L, morethan5(i)=FORALL(j=1..4, coffe[i,j]>5)
Pre: -
Post: (van,ind)=SEARCH(i=1..n, morethan5(i))
```

```yaml
n: 3
coffe: [[4, 5, 7, 5], [6, 8, 7, 9], [4, 5, 7, 5]]
van: true
ind: 2
```

```yaml
# not found: ind may be 0, 4, …
n: 3
coffe: [[4, 3, 5, 6], [4, 7, 5, 6], [5, 6, 5, 6]]
van: false
ind: 0
```

The original course screenshot names the flag `exists`, which the editor rejects. The flag is renamed `van` here.

### Task 5: Dogbert's star (conditional maximum, skipped slot)

> *Brightness measured at regular intervals; values ≤ k mean clouds. Give the largest change between two consecutive clear-sky measurements.*

```text
In: k∈N, data∈Z[1..]
Out: van∈L, maxval∈Z
Pre: -
Post: (van,,maxval)=CONDMAX(i=1..length(data)-1,
      data[i]-data[i+1], data[i]>k and data[i+1]>k)
```

```yaml
k: 10
data: [34, 30, 1, 2, 1, 30, 33, 27, 30, 32]
van: true
maxval: 6
```

```yaml
# never two clear measurements in a row: maxval is unconstrained
k: 50
data: [34, 30, 1, 2, 1, 30, 33, 27, 30, 32]
van: false
maxval: 0
```

The clear pairs give 4, −3, 6, −3 and −2, so the maximum is 6 at i=7.

### Task 6: Travel agency (Hungarian relational style)

> *An agency sells tickets to cities at given distances and prices. Give the price of the ticket to the farthest city; if several are equally far, the cheapest. 1 ≤ N ≤ 100, distance 1 ≤ T ≤ 20 000, price 1 ≤ Á ≤ 2 000 000.*

```text
Be:
n∈N,
ar∈N[1..n],
tav∈N[1..n]

Ki:
max∈N

Ef:
1<=n and n<=100 and ∀i∈[1..n]:(1<=ar[i] and ar[i]<=2000000 and 1<=tav[i] and tav[i]<=20000)

Uf:
∃i∈[1..n]:(∀j∈[1..n]:(tav[j]<tav[i] vagy (tav[j]=tav[i] és ar[j]>=ar[i])) és ar[i]=max)
```

```yaml
n: 6
ar: [30000, 200000, 150000, 38000, 150000, 30000]
tav: [50, 1900, 2000, 900, 600, 2100]
max: 30000
```

```yaml
# tie on distance: the cheaper one
n: 3
ar: [500, 250, 100]
tav: [1000, 1000, 500]
max: 250
```

```yaml
n: 4
ar: [100, 100, 2000000, 1]
tav: [20000, 20000, 20000, 20000]
max: 1
```

There is a city `i` such that every city is nearer, or equally far but not cheaper, and the output is its price. The output may be called `max`, because the macro is the uppercase `MAX`. The macro-style equivalent with records:

```text
Be: n∈N, varos∈Varos[1..n], Varos=(tav:N x ar:N)
Sa: maxTav∈N, db∈N, arak∈N[1..db]
Ki: jegyAr∈N
Ef: n>0
Uf: (,maxTav)=MAX(i=1..n, varos[i].tav) és
    (db,arak)=KIVÁLOGAT(i=1..n, varos[i].tav=maxTav, varos[i].ar) és
    (,jegyAr)=MIN(i=1..db, arak[i])
```

```yaml
n: 3
varos: [{tav: 1000, ar: 500}, {tav: 1000, ar: 250}, {tav: 500, ar: 100}]
maxTav: 1000
db: 2
arak: [500, 250]
jegyAr: 250
```

Auxiliary variables have to be in the test case too, which makes the relational form handier here.

### Task 7: Number of divisors (`DARAB`)

```text
Be: szam∈N
Ki: osztok∈N
Ef: szam>=1
Uf: osztok=DARAB(i=1..szam, i|szam)
```

```yaml
szam: 12
osztok: 6
```

```yaml
szam: 13
osztok: 2
```

### Task 8: Sum of the odd divisors (filtered `SUM`)

```text
Be: n∈N
Ki: osszeg∈N
Ef: n>=1
Uf: osszeg=SZUMMA(i=1..n, i, i mod 2 = 1 és i|n)
```

```yaml
n: 10
osszeg: 6
```

```yaml
n: 1
osszeg: 1
```

A version seen in course material, `SUM(i=1..n, i, i%2=1)`, sums **every** odd number up to n (25 for n=10), not the odd divisors. The filter must hold every restriction in the statement.

### Task 9: Card values (constant lookup table)

> *How much is a Hungarian card worth in 21? Cards `also, felso, kiraly, VII, VIII, IX, X, asz` are worth `2, 3, 4, 7, 8, 9, 10, 11`.*

```text
Be:
lapok∈S[1..8]=["also","felso","kiraly","VII","VIII","IX","X","asz"],
ertekek∈N[1..8]=[2,3,4,7,8,9,10,11],
lap∈S

Ki:
ertek∈N

Ef:
lap!="" és ∃i∈[1..8]:(lapok[i]=lap)

Uf:
∃i∈[1..8]:(lapok[i]=lap és ertek=ertekek[i])
```

```yaml
lap: asz
ertek: 11
```

```yaml
lap: "also"
ertek: 2
```

`lapok` and `ertekek` are constants: they appear in `Uf:` (the mandate) but **not** in the test data.

### Task 10: Christmas trees (first price below K, or −1)

> *N trees with prices. Give the 1-based index of a tree cheaper than K forints; the smallest such index; −1 if none. 1 ≤ N ≤ 100, 1 ≤ K ≤ 10 000, 1 ≤ price ≤ 10 000.*

Relational:

```text
Be:
n∈N,
k∈N,
ar∈N[1..n]

Ki:
sorszam∈Z

Ef:
1<=n<=100 és 1<=k<=10000 és ∀i∈[1..n]:(1<=ar[i] és ar[i]<=10000)

Uf:
(sorszam=-1 és ∀i∈[1..n]:(ar[i]>=k)) vagy
∃i∈[1..n]:(sorszam=i és ar[i]<k és ∀j∈[1..i-1]:(ar[j]>=k))
```

```yaml
n: 6
k: 2000
ar: [2500, 5300, 1900, 2400, 8800, 5300]
sorszam: 3
```

```yaml
n: 3
k: 100
ar: [2500, 5300, 1900]
sorszam: -1
```

```yaml
# smallest index, not cheapest
n: 4
k: 5000
ar: [6000, 3000, 1000, 200]
sorszam: 2
```

Macro style, using every Hungarian block:

```text
Be:
n∈N,
k∈N,
ar∈N[1..n]

Sa:
van∈L,
ind∈N

Ki:
sorszam∈Z,
db∈N

Fv:
kisebb:N->L,
kisebb(i)=ar[i]<k

Ef:
1<=n<=100 és 1<=k<=10000 és ∀i∈[1..n]:(1<=ar[i] és ar[i]<=10000)

Uf:
(van,ind)=KERES(i=1..n, kisebb(i)) és
(van -> sorszam=ind) és
(nem van -> sorszam=-1) és
db=DARAB(i=1..n, ar[i]<k)
```

```yaml
n: 6
k: 2000
ar: [2500, 5300, 1900, 2400, 8800, 1000]
van: true
ind: 3
sorszam: 3
db: 2
```

```yaml
n: 3
k: 100
ar: [2500, 5300, 1900]
van: false
ind: 4
sorszam: -1
db: 0
```

- `sorszam` is `Z` because `-1` is not in `N`.
- `∀j∈[1..i-1]:(ar[j]>=k)` makes the witness the **first** match.
- The sentinel is stated in `Uf:`; KERES never produces one.

### Task 11: Distinct times (backward FORALL in a function)

```text
In: n∈N, races∈Race[1..n], Race=(name:S x time:N)
Out: cnt∈N
Fn: nV:N->L, nV(i)=FORALL(j=1..i-1, races[i].time≠races[j].time)
Pre: -
Post: cnt=COUNT(i=1..n, nV(i))
```

```yaml
n: 4
races:
  - name: AS
    time: 37
  - name: BE
    time: 31
  - name: FG
    time: 30
  - name: PS
    time: 37
cnt: 3
```

`nV(i)` is true when `races[i].time` did not occur before. For `i=1` the range `1..0` is empty, so FORALL is true.

### Task 12: Long runs (3-argument MIS, record field)

```text
In: Trun=(time:N x loc:S), n∈N, runs∈Trun[1..n]
Out: cnt∈N, location∈S[1..cnt]
Pre: -
Post: (cnt,location)=MIS(i=1..n, runs[i].time>30, runs[i].loc)
```

```yaml
n: 4
runs: [{time: 15, loc: office}, {time: 35, loc: restaurant}, {time: 40, loc: gym}, {time: 5, loc: shop}]
cnt: 2
location: [restaurant, gym]
```

### Task 13: Sessions where everyone was late (column FORALL, 2-argument MIS)

```text
In: n∈N, m∈N, k∈N, lates∈N[1..n,1..m]
Out: cnt∈N, every∈N[1..cnt]
Fn: lt:N->L, lt(j)=FORALL(i=1..n, lates[i,j]>k)
Pre: -
Post: (cnt,every)=MIS(j=1..m, lt(j))
```

```yaml
n: 3
m: 4
k: 5
lates: [
  [6, 8, 2, 7],
  [7, 9, 4, 8],
  [6, 6, 1, 9]
]
cnt: 3
every: [1, 2, 4]
```

The two-argument MIS collects the column **indexes**.

### Task 14: First odd number (SELECT)

```text
In: n∈N, nums∈Z[1..n]
Out: ind∈N
Pre: ∃i∈[1..n]:(nums[i] mod 2 = 1)
Post: ind=SELECT(i>=1, nums[i] mod 2 = 1) and n=n
```

```yaml
n: 5
nums: [24, 26, 23, 28, 29]
ind: 3
```

### Task 15: Profit, when there is one (implication around a conditional maximum)

> *For each product: purchase price `b`, shop price `a`, sale price `k`, units sold at shop price `aDb` and at sale price `kDb`. Among products that sold more at the sale price, give the largest profit `(k-b)*kDb`, or 0 if there is none.*

```text
Be:
n∈N,
t∈Termek[1..n],
Termek=(b:N x a:N x k:N x aDb:N x kDb:N)

Sa:
van∈L

Ki:
maxhaszon∈N

Ef:
1<=n<=100 és ∀i∈[1..n]:(t[i].b<=t[i].k<=t[i].a)

Uf:
(van -> (van,,maxhaszon)=FELTMAX(i=1..n, (t[i].k-t[i].b)*t[i].kDb, t[i].kDb>t[i].aDb)) és
(nem van -> maxhaszon=0)
```

```yaml
n: 2
t: [{b: 9, a: 10, k: 9, aDb: 1, kDb: 8}, {b: 6, a: 10, k: 8, aDb: 4, kDb: 8}]
van: true
maxhaszon: 16
```

```yaml
n: 1
t: [{b: 1, a: 2, k: 1, aDb: 5, kDb: 1}]
van: false
maxhaszon: 0
```

---

## 12. Pitfalls and error messages

| # | Symptom / message | Cause and fix |
|:--:|:--|:--|
| 1 | "Missing space after `Ef:`" | `Ef:-` → `Ef: -`, or put the content on the next line. |
| 2 | "Unknown block header `Segéd:`", online `Expected ",", "Aux:", "Ki:", "Out:", "Sa:", …` | Invented or translated header. The real set is `Be Sa Ki Fv Ef Uf` / `In Aux Out Fn Def Pre Post`. |
| 3 | "Unknown block header `in:` … case-sensitive" | Write `In:`. |
| 4 | "`Ki:` is in the wrong place" | Order: declarations → `Fv` → `Ef` → `Uf`. |
| 5 | "Missing `Ef:` block" | Add `Ef: -`. |
| 6 | "Second `Be:` block" | Merge the blocks with commas. |
| 7 | "Trailing comma" | No comma after a block's last declaration. |
| 8 | "Put a comma at the end of the previous declaration" | Layout B needs a comma at each line end except the last. |
| 9 | "`exists` is a reserved word", online `incomplite type for exists: undefined is not boolean` | Rename the variable to `van`/`talalt` in the declaration, the formula **and** the test data. The same goes for `forall`, `in`, `not`, `mod`, `div`, `igaz`, … |
| 10 | "Unexpected character `→`" (or `∧`, `∨`) | Write `->`, `and`/`és`, `or`/`vagy`. |
| 11 | "Expected `>=` but found `≥`" | Inside SELECT only `>=` works. |
| 12 | "Only one function per `Fn:` header" | One function per `Fv:` line; repeat the header. |
| 13 | "`f` has no signature" | Write `f:N->N, f(i)=…`. |
| 14 | "`MAX` gives 2 values … but the signature of `f` has 1 in its codomain" | Codomain `N x N` and use `f(i).2`, or `f(i)=MAX(…).2`. |
| 15 | "Unknown function `ABS`" / "Unknown function `sum`" | Built-ins are lowercase (`abs`); macros are uppercase (`SUM`). |
| 16 | "Use `∈` to declare a variable" / "`element`" error | Declarations are `x∈T` (or `x in T`, `x eleme T`). `element` is not a keyword. |
| 17 | "Use `=` instead of `:=`" | `:=` exists only in structograms. |
| 18 | "Record fields are separated by ` x `, not commas" | `(nev:S x kor:N)`. |
| 19 | "Input `x` is never used in the postcondition", online `x is missing from the post condition!` | Use it in `Uf:` or append `and x=x`. This applies to aux, outputs and constants too. |
| 20 | "Output `x` is used in the precondition", online `Output data x is in the precondition!` | Move that condition to `Uf:`. |
| 21 | "`x` is not declared … Did you mean" | Typo, or a missing declaration. |
| 22 | "Auxiliary variable `ind` is missing from this test case" | Every non-constant variable needs a value; any valid value for a don't-care. |
| 23 | `data cannot contain item with the same name: lapok` | Delete constants from the test data. |
| 24 | "`sorszam` is -1, but it is declared as N" | Declare it `Z`. |
| 25 | "`ar` has 2 element(s) … so it needs exactly 3" | The array length must match the declared range. |
| 26 | "`t[i]` should be true or false" after a quantifier | Parenthesize the body: `∀i∈[1..n]:(t[i]>0)`. |
| 27 | "index 6 is out of range" | A loop that reads `t[i+1]` must stop at `n-1`; `0..`-based arrays loop over `0..length(t)-1` (or `from(t)..to(t)`). |
| 28 | "MAX over an empty range (1..0)" | Require `n>0` in `Ef:`, or use `FELTMAX`. |
| 29 | "`ind` has no defined value" | A failed search's index was compared or used as an index. Guard it with `van -> …`. |
| 30 | "The two arrays don't have the same index range" | For `y=COPY(i=0..n-1,…)`, declare `y∈Z[0..]`; MIS results start at 1. |
| 31 | A green test whose specification is still wrong | The filter is missing a restriction (Task 8), a bound is too loose (section 7), or the first match is not enforced (Task 10). Add test cases that separate the wrong reading from the right one. |
| 32 | Mixed languages (`Be:` over `nums`, `cnt`) | Hungarian headers with Hungarian names, or English with English. |

---

## 13. Grammar

The grammar of speci-local's parser. `{x}` means zero or more, `[x]` means optional.

```ebnf
document    = { comment } block { block } ;            (* order checked separately *)
block       = header ( " " | newline ) content ;       (* content runs to the next header *)
header      = ( "Be" | "In" | "Sa" | "Aux" | "Ki" | "Out"
              | "Fv" | "Fn" | "Def" | "Ef" | "Pre" | "Uf" | "Post" ) ":" ;

(* Be/Sa/Ki *)
decls       = decl { "," decl } ;                      (* no trailing comma *)
decl        = name ( "∈" | "in" | "eleme" ) type [ "=" expr ]   (* "=" makes a constant *)
            | TypeName "=" type ;                      (* alias *)
type        = base { "[" dims "]" } ;
base        = "N" | "Z" | "R" | "L" | "S" | "C" | "K" | TypeName
            | "[" sum ".." sum "]"                     (* interval *)
            | "(" field { ( "x" | "*" ) field } ")" ;  (* record *)
field       = name ":" type ;
dims        = [ dim ] { "," [ dim ] } ;                (* "[]" = "[1..]", "[,]" = "[1..,1..]" *)
dim         = sum ".." [ sum ] ;

(* Fv: exactly one function *)
function    = name ":" types "->" types "," name "(" [ name { "," name } ] ")" "=" expr ;
types       = type { ( "x" | "*" ) type } ;

(* Ef/Uf *)
condition   = "-" | expr ;
expr        = conj { ( "or" | "vagy" ) conj } ;
conj        = impl { ( "and" | "és" ) impl } ;
impl        = negation [ "->" impl ] ;
negation    = ( "not" | "nem" | "¬" ) negation | comparison ;
comparison  = sum ( ( "∈" | "in" | "eleme" ) "[" sum ".." sum "]"
                  | { relop sum } ) ;
relop       = "=" | "≠" | "!=" | "<>" | "<" | ">" | "≤" | "<=" | "≥" | ">=" | "|" | "∤" ;
sum         = product { ( "+" | "-" ) product } ;
product     = unary { ( "*" | "/" | "%" | "mod" | "div" ) unary } ;
unary       = ( "-" | "not" | "nem" ) unary | postfix ;
postfix     = atom { "[" expr { "," expr } "]" | "." ( name | digits ) | "!" } ;
atom        = number | text | "true" | "igaz" | "false" | "hamis"
            | name [ "(" [ expr { "," expr } ] ")" ]          (* variable or call *)
            | MACRO "(" name ( "=" sum ".." sum | ">=" sum ) { "," expr } ")"
            | "(" [ expr ] { "," [ expr ] } ")"               (* group or tuple *)
            | "[" [ sum ".." { "," sum ".." } ":" ] [ expr { "," expr } ] "]"
            | "{" expr [","] ( "ha" | "if" ) expr { ";" expr [","] ( "ha" | "if" ) expr }
                  [ ";" expr [","] ( "egyébként" | "otherwise" ) ] "}"
            | ( "∀" | "forall" | "minden" | "∃" | "exists" | "létezik" )
                  name ( "∈" | "in" | "eleme" ) "[" sum ".." sum "]" ":" unary ;
text        = '"' … '"' | "'" … "'" | "„" … "”" ;     (* one line *)
comment     = "//" … newline | "/*" … "*/" ;
```

`MACRO` is one of the names in the table in 8.2. Only SELECT uses the `>=` form, and only with `>=`. The `x∈[a..b]` membership form allows no chained comparison after it.

---

## 14. Checklist before answering

1. **Headers:** exact spelling, a space or line break after the colon, Hungarian by default, one language per file.
2. **Order:** `Be`/`Sa`/`Ki` → `Fv` (one per header) → `Ef` → `Uf`. `Ef: -` if empty. No trailing comma.
3. **Names:** Hungarian, no reserved words (`exists`, `in`, `nem`, …), types capitalised.
4. **Types:** `Z` for anything that can be negative (sentinels), interval types where the statement bounds a value, `C`/`K` for single characters, `[1..n]` arrays matching the input format.
5. **Operators:** `->` not `→`, `abs` not `ABS`, `SELECT(i>=…)` not `≥`, quantifier bodies in parentheses, implications parenthesized when joined with `és`/`vagy`.
6. **Theorems:** argument order (`FELTMAX(i=…, value, condition)`), slot counts (`(van,,ertek)`), MIS gives indexes unless a third argument is given, MAX needs a non-empty range, `KERES`'s index guarded by `van -> …`, sentinel values stated in `Uf:`.
7. **Functions:** signature first with `->`, the tuple codomain for MAX/KERES/KIVÁLOGAT/FELTMAX bodies.
8. **Mandate:** every declared variable (aux, outputs and constants too) occurs in `Uf:`; no outputs in `Ef:`.
9. **Bounds:** each `Ef:` bound copied from the statement and re-read; loops reading `t[i+1]` stop at `n-1`.
10. **Test cases:** every non-constant variable present (aux included); no constants; array lengths match; `true`/`false` for `L`; at least one normal case, one "not found"/empty case, and one case that separates "first" from "best" or "any" when the statement says which.
