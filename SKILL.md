---
name: specifikacio
description: "Write, check and debug specifications for the ELTE PROGRAMMING course (Programozás, IP-18fPROGEG) in the Specification Editor (progalap.elte.hu/specifikacio) and its offline twin speci-local, and solve whole exercises: reduction to a programming template (sablon, mintavisszavezetés), the specification (Be:/Sa:/Ki:/Fv:/Ef:/Uf: or In:/Aux:/Out:/Fn:/Pre:/Post:) with 3 YAML test cases, the structogram (Stuki) in the same .sp file, and the C# program. Covers the programming theorems (SZUMMA, DARAB, MAX, MIN, FELTMAX, KERES, VAN, MIND, KIVÁLASZT, MÁSOL, KIVÁLOGAT), ∀/∃ conditions, functions and the editors' error messages. Use it whenever the user mentions specifikáció, struktogram, Stuki, sablon, visszavezetés, programozási tételek or minták, progalap, speci-local or .sp/.st files, pastes Be:/Ki:/Ef:/Uf: text, or asks for an ELTE-style solution in C#, even without naming the tool."
---

# ELTE specifications, structograms and C#

The ELTE IK course **PROGRAMMING** (*Programozás*, `IP-18fPROGEG`) solves every task the same way: recognise a **programming template** (*sablon*, a *programozási tétel*), write the **specification** as that template with the task's details substituted, check it on **test cases**, turn the template's algorithm into a **structogram**, and code it in **C#**.

- The **Specification Editor** (*Specifikáció szerkesztő*, [progalap.elte.hu/specifikacio](https://progalap.elte.hu/specifikacio/)) evaluates the precondition and postcondition on each test case and marks it green or red.
- **Stuki** ([progalap.elte.hu/stuki](https://progalap.elte.hu/stuki/)) draws structograms.
- **speci-local** is an offline reimplementation of both in one app. It accepts and rejects the same specifications and judges test cases the same way as the online editor (checked on 816 cases), and it runs structograms on the specification's test cases.

Everything in this manual was checked against speci-local's parser, evaluator and structogram runner. Where the online editor's own error text is known, it is quoted.

## Contents

1. [What to produce](#1-what-to-produce)
2. [Quick rules](#2-quick-rules)
3. [Programming templates and reduction](#3-programming-templates-and-reduction)
4. [The specification language](#4-the-specification-language)
5. [Test cases](#5-test-cases)
6. [Structograms](#6-structograms)
7. [C#](#7-c)
8. [A complete exercise](#8-a-complete-exercise)
9. [Case studies](#9-case-studies)
10. [Pitfalls and error messages](#10-pitfalls-and-error-messages)
11. [The tools](#11-the-tools)
12. [Grammar](#12-grammar)
13. [Checklist](#13-checklist)

---

## 1. What to produce

### 1.1. Write for understanding

The reader is learning the method, so a solution that is easy to follow beats a short one. The course's method is itself the best explanation: a reader who knows the templates recognises a solution built from them at a glance.

- **Use the templates literally:** their names (Keresés, Megszámolás, …), their variable names (`van`, `ind`, `db`, `s`, `maxind`, `maxert`, `y`) and their algorithms, element by element (section 3.3). Don't merge steps or invent a cleverer loop.
- **One idea per line:** one declaration per line (layout B), one theorem per line of `Uf:`, one statement per structogram box, one statement per C# line.
- **Names that say what they hold:** `sorszam`, `maxTav`, `jegyAr`, never `x2`, `tmp`, `res`. Plain ASCII names (`maxert`, `ertek`) are safest, since they are typed in three places (specification, structogram, C#).
- **Name a condition or value when it helps the reading:** if `T(i)` or `f(i)` is more than a simple comparison, or appears twice, give it a function with a telling name (`Fv: tiszta:N->L, tiszta(i)=adat[i]>k és adat[i+1]>k`). Then the reduction reads like the statement: `T(i) ~ tiszta(i)`. A nested theorem always gets a function (3.5).
- **Prefer the template (macro) form** (`(van,ind)=KERES(…)`) over the relational ∀/∃ form in a full exercise, because it maps one-to-one onto the algorithm. Use the relational form when the user asks for it.
- **No tricks.** When one can't be avoided (the reverse search in 3.4), say in a comment what it does.
- **Explain in a sentence or two** which template fits and why ("we need the first cheap tree, and there may be none: that is Keresés"), in the user's language.

### 1.2. A specification

When asked to write or fix a specification, answer in this layout, unless the user's file or request uses another (layout A, English headers):

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
# három páratlan szám van: 3, 81 és 5
n: 5
szamok: [3, 72, 81, 4, 5]
db: 3
```

Give test cases with it: the three kinds in 5.3 unless the user wants fewer.

### 1.3. A full exercise

A **full exercise** (*teljes feladat*) is four parts, all for the same task:

1. **The reduction** (*visszavezetés*, 3.5): which template the task is, and what each part of the template means here.
2. **The specification with 3 test cases**: the template's short form with the reduction substituted (sections 2–5).
3. **The structogram**: the template's algorithm with the same substitution (section 6).
4. **The C# program**: the structogram translated line by line (section 7).

Work in this order:

1. Read the statement for: the inputs and outputs, every bound (they go to `Ef:`), what exactly is asked (first, largest, all, how many, whether one exists), what happens when there is no answer (a `-1`, a `0`, nothing), and the input and output format.
2. Choose the template (3.1) and write the reduction.
3. Write the specification; check it against the checklist (13).
4. Write the 3 test cases and work out every expected value by hand, step by step. The tests are only as good as these values.
5. Write the structogram: the template's algorithm, then the rest of `Uf:` (6.1).
6. Translate it into C#.

Answer with these parts, in this order:

1. **Visszavezetés**: one or two sentences, then the mapping block (3.5).
2. **The `.sp` file**: the specification, the 3 tests and the structogram in one file (11.2), so it opens in speci-local with everything in place. Section 8 is a complete one.
3. **The structogram drawn** (6.5), so it can be read without the app.
4. **The C# program.**

When working in a directory, write `<name>.sp` and `<name>.cs`. If speci-local or its source is at hand, open the `.sp` and check that the tests are green on both tabs; if `dotnet` is available, build and run the C# on the tests' inputs.

### 1.4. Fixing or explaining a specification

1. Read the error message and its position, then find it in section 10.
2. Fix the cause, not the symptom (a missing `Ef:` bound, a wrong template argument, a variable that should be `Z`), and keep the user's style: language, layout and names.
3. Re-check the mandate (every variable in `Uf:`) and the test cases (every variable has a value).
4. Say in a sentence what was wrong and why the fix is right.

---

## 2. Quick rules

1. **Blocks and order.** Declarations first (`Be:`, `Sa:`, `Ki:` in any order among themselves), then functions (`Fv:`, one function per header), then `Ef:`, then `Uf:`. `Be:`, `Ki:`, `Ef:` and `Uf:` are compulsory; write `Ef: -` when there is no precondition.
2. **Headers are exact and case-sensitive:** `Be: Sa: Ki: Fv: Ef: Uf:` or `In: Aux: Out: Fn: Pre: Post:` (`Def:` is another function header). After the colon comes a space (content on the same line) or a line break. `Segéd:`, `in:`, `Pre :` and `Ef:-` are all errors.
3. **Hungarian by default:** Hungarian headers *and* Hungarian identifiers (`szam`, `ar`, `tav`, `db`, `van`, `ind`, `sorszam`, `ertek`) unless the user writes in English or edits an English file. Never silently translate a working file from one style to the other.
4. **Every declared variable must appear in the postcondition**: inputs, auxiliary variables, outputs and constants. Appearing only in `Ef:` does not count. Add `és x=x` for one that the formula really doesn't need.
5. **Every declared variable needs a value in every test case**, auxiliary ones too (`van`, `ind`), except constants declared with an initial value: those must *not* appear in the test data.
6. **ASCII where the glyph is refused:** implication is `->` (the glyph `→` is rejected everywhere), function arrows are `->`, and inside `SELECT`/`KIVÁLASZT` only `>=` works (`KIVÁLASZT(i>=1, …)`). `∈ ∀ ∃ ≤ ≥ ≠` are fine elsewhere; `<= >= !=` work too.
7. **Reserved words are not names:** `exists`, `forall`, `in`, `and`, `or`, `not`, `mod`, `div`, `true`, `false` and the Hungarian `létezik`, `minden`, `eleme`, `és`, `vagy`, `nem`, `igaz`, `hamis`. Call a search flag `van` or `talalt`, never `exists`.
8. **Theorem macros are uppercase** (`SZUMMA`, `MAX`, `KERES`, `FELTMAX`, …), so lowercase variables like `max`, `sum`, `db` never clash with them. Built-in functions are lowercase (`abs`, not `ABS`).
9. **An output that can be `-1` is typed `Z`**, never `N`.
10. **Parenthesize quantifier bodies:** `∀i∈[1..n]:(t[i]>0)`. Without the parentheses only `t[i]` is the body.

---

## 3. Programming templates and reduction

The course (*Programozási minták*, Horváth Győző) gives each pattern as a **template** (*sablon*): an abstract task over an interval `[e..u]` of whole numbers, with a condition `T(i)` and/or a function `f(i)`, together with its full specification, its short form and its algorithm. A task is solved by **reducing** it to a template (*mintavisszavezetés*): saying what `e..u`, `T(i)` and `f(i)` are in this task, then substituting.

### 3.1. Choosing the template

| The task asks… | Template | Short form |
|:--|:--|:--|
| the sum (product, …) of values | Összegzés | `s=SZUMMA(i=e..u, f(i))` |
| how many satisfy a condition | Megszámolás | `db=DARAB(i=e..u, T(i))` |
| the largest / smallest value and where it is | Maximum- / Minimumkiválasztás | `(maxind,maxert)=MAX(i=e..u, f(i))` |
| the largest / smallest among those satisfying a condition | Feltételes maximum- / minimumkeresés | `(van,maxind,maxert)=FELTMAX(i=e..u, f(i), T(i))` |
| the first one satisfying a condition, if there is one | Keresés | `(van,ind)=KERES(i=e..u, T(i))` |
| whether there is one | Eldöntés | `van=VAN(i=e..u, T(i))` |
| whether all of them satisfy it | Mind eldöntés | `mind=MIND(i=e..u, T(i))` |
| the first one, known to exist | Kiválasztás | `ind=KIVÁLASZT(i>=e, T(i))` |
| a value for every element | Másolás | `y=MÁSOL(i=e..u, f(i))` |
| all of those satisfying a condition | Kiválogatás | `(db,y)=KIVÁLOGAT(i=e..u, T(i), f(i))` |

- **"The largest" when the range may be empty** (`n` may be 0, or nothing qualifies): Feltételes maximumkeresés, because Maximumkiválasztás needs a non-empty range.
- **"The first one"** when the statement guarantees it exists (and says so, or the precondition can say so): Kiválasztás. Otherwise Keresés.
- **The last one:** reverse Keresés (3.4).
- **Nested** (a maximum of row sums, all elements of a row, a sum per window): reduce the inner task to a template too, and make it a function (3.5).

### 3.2. Names and arguments

Every alias behaves exactly like its main name. Macro names are uppercase; a lowercase `sum(…)` is an unknown function.

| Template | Main name | Other names | Arguments after the range | Result |
|:--|:--|:--|:--|:--|
| Összegzés | `SUM` | `SZUMMA`, `SZUM` | `f(i)` [, `T(i)`] | number (0 over an empty range) |
| Megszámolás | `COUNT` | `DARAB`, `MEGSZÁMOL` | `T(i)` | natural number |
| Maximumkiválasztás | `MAX` | — | `f(i)` | `(ind, value)`, the **first** maximum |
| Minimumkiválasztás | `MIN` | — | `f(i)` | `(ind, value)`, the first minimum |
| Eldöntés | `EXISTS` | `VAN`, `ELDÖNT`, `LÉTEZIK`, `DECISION` | `T(i)` | `L` |
| Mind eldöntés | `FORALL` | `MIND`, `EVERY` | `T(i)` | `L` |
| Kiválasztás | `SELECT` | `KIVÁLASZT` | written `KIVÁLASZT(i>=e, T(i))` | index |
| Keresés | `SEARCH` | `KERES` | `T(i)` | `(found, ind)` |
| Másolás | `COPY` | `MÁSOL`, `MAP` | `f(i)` | array indexed `e..u` |
| Kiválogatás | `MIS` | `KIVÁLOGAT`, `FILTER` | `T(i)` [, `f(i)`] | `(count, array indexed from 1)` |
| Feltételes maximumkeresés | `CONDMAX` | `FELTMAX`, `MAXHA`, `MAXIF` | `f(i), T(i)` | `(found, ind, value)` |
| Feltételes minimumkeresés | `CONDMIN` | `FELTMIN`, `MINHA`, `MINIF` | `f(i), T(i)` | `(found, ind, value)` |

A wrong argument count is an error, for example "DARAB got 2 argument(s) after the range, but needs 1".

### 3.3. The templates

Each template: its specification (`e∈Z, u∈Z` are the template's inputs; a real task replaces them), its short form, and its algorithm. The algorithms are written as an **outline**, one structogram element per line:

| Outline | Structogram element |
|:--|:--|
| `x:=…` | statement |
| `LOOP text` + indented body | pre-test loop; `i=e..u` makes it a count loop, anything else is the loop condition |
| `DO` + indented body + `WHILE c` | post-test loop: repeats while `c` holds |
| `IF c` / `ELSE` + indented bodies | decision |
| `SWITCH` + `CASE c` + indented bodies | multi-way decision |
| `-` | the empty statement |

`i` is the template's local loop variable (*Változó i:Egész*). All of these algorithms were run in speci-local's Stuki tab on test cases, and they are what its **From spec** button generates.

**Összegzés** (sum). Over an empty range the sum is 0.

```text
Ki: s∈H                          s:=0
Uf: s=SZUMMA(i=e..u, f(i))       LOOP i=e..u
                                   s:=s+f(i)
```

**Megszámolás** (count). It is the sum of 1 over the elements satisfying `T`: `db=SZUMMA(i=e..u, 1, T(i))`.

```text
Ki: db∈N                         db:=0
Uf: db=DARAB(i=e..u, T(i))       LOOP i=e..u
                                   IF T(i)
                                     db:=db+1
                                   ELSE
                                     -
```

**Maximumkiválasztás** (maximum). Needs a non-empty range, so `Ef: e<=u`. Ties go to the first index, because only a strictly greater value replaces the current one.

```text
Ki: maxind∈Z, maxert∈H           maxert:=f(e)
Ef: e<=u                         maxind:=e
Uf: maxind∈[e..u] és             LOOP i=e+1..u
    ∀i∈[e..u]:(f(maxind)>=f(i))    IF f(i)>maxert
    és maxert=f(maxind)              maxert:=f(i)
Short: (maxind,maxert)=              maxind:=i
       MAX(i=e..u, f(i))           ELSE
                                     -
```

**Minimumkiválasztás** (minimum): the same with `MIN`, `minind`, `minert` and `f(i)<minert`.

**Feltételes maximumkeresés** (conditional maximum). Finds the largest `f(i)` among the `i` satisfying `T(i)`; `van` says whether there was any. The algorithm's three cases: `i` doesn't qualify; it qualifies and there is already a candidate (compare); it is the first that qualifies (take it).

```text
Ki: van∈L, maxind∈Z, maxert∈H    van:=hamis
Uf: van=∃i∈[e..u]:(T(i)) és      LOOP i=e..u
    van -> (maxind∈[e..u] és       SWITCH
      maxert=f(maxind) és            CASE nem T(i)
      T(maxind) és                     -
      ∀i∈[e..u]:(T(i) ->             CASE van és T(i)
        maxert>=f(i)))                 IF f(i)>maxert
Short: (van,maxind,maxert)=              maxert:=f(i)
       FELTMAX(i=e..u, f(i), T(i))       maxind:=i
                                       ELSE
                                         -
                                     CASE nem van és T(i)
                                       van:=igaz
                                       maxert:=f(i)
                                       maxind:=i
```

**Feltételes minimumkeresés**: the same with `FELTMIN`, `minind`, `minert` and `f(i)<minert`.

**Keresés** (linear search). Finds the first `i` satisfying `T(i)`, if there is one.

```text
Ki: van∈L, ind∈Z                 i:=e
Uf: van=∃i∈[e..u]:(T(i)) és      LOOP i≤u és nem T(i)
    van->(ind∈[e..u] és T(ind)     i:=i+1
      és ∀i∈[e..ind-1]:          van:=i≤u
        (nem T(i)))              IF van
Short: (van,ind)=                  ind:=i
       KERES(i=e..u, T(i))       ELSE
                                   -
```

The slides show three equivalent variants: `ind` itself as the loop variable (`ind:=e`, `LOOP ind≤u és nem T(ind)`, `van:=ind≤u`), a flag loop (`van:=hamis`, `ind:=e`, `LOOP nem van és ind≤u` with `IF T(ind)` → `van:=igaz`, `ELSE` → `ind:=ind+1`), and `van:=T(i)`, `ind:=i`, `i:=i+1` inside a `nem van és i≤u` loop. All are correct; use the first form above unless the user's course material uses another.

**Eldöntés** (decision): is there an `i` satisfying `T(i)`?

```text
Ki: van∈L                        i:=e
Uf: van=∃i∈[e..u]:(T(i))         LOOP i≤u és nem T(i)
Short: van=VAN(i=e..u, T(i))       i:=i+1
                                 van:=i≤u
```

**Mind eldöntés** (all): does every `i` satisfy `T(i)`? Over an empty range, yes.

```text
Ki: mind∈L                       i:=e
Uf: mind=∀i∈[e..u]:(T(i))        LOOP i≤u és T(i)
Short: mind=MIND(i=e..u, T(i))     i:=i+1
                                 mind:=i>u
```

**Kiválasztás** (selection): the first `i ≥ e` satisfying `T(i)`, **known to exist**. There is no upper bound, so the precondition must guarantee a match.

```text
Ki: ind∈Z                        i:=e
Ef: ∃i∈[e..∞]:(T(i))             LOOP nem T(i)
Uf: ind>=e és T(ind) és            i:=i+1
    ∀i∈[e..ind-1]:(nem T(i))     ind:=i
Short: ind=KIVÁLASZT(i>=e, T(i))
```

**Másolás** (copy): `f(i)` for every `i`.

```text
Ki: y∈H[e..u]                    LOOP i=e..u
Uf: ∀i∈[e..u]:(y[i]=f(i))          y[i]:=f(i)
Short: y=MÁSOL(i=e..u, f(i))
```

**Kiválogatás** (multiple selection): the `f(i)` values of the `i` satisfying `T(i)`, in order. Without the third argument it collects the indexes `i` themselves.

```text
Ki: db∈N, y∈H[1..db]             db:=0
Uf: db=DARAB(i=e..u, T(i)) és    LOOP i=e..u
    ∀i∈[1..db]:(∃j∈[e..u]:         IF T(i)
      (T(j) és y[i]=f(j)))           db:=db+1
    and y keeps the order            y[db]:=f(i)
Short: (db,y)=                     ELSE
  KIVÁLOGAT(i=e..u, T(i), f(i))      -
```

**Általános összegzés** (general summation) is a way of thinking from the slides, not a macro: `SZUMMA(i=e..u, f(i), kezd, add)` folds `f(e)…f(u)` into a starting value `kezd` with an operation `add` (`+` with 0, `*` with 1, `vagy` with `hamis`, appending with the empty array, …). Its algorithm is `s:=kezd`, `LOOP i=e..u`, `s:=add(s,f(i))`. Neither editor accepts the four-argument form ("SUM got 3 argument(s) after the range, but needs 1 or 2"); use the specific template.

### 3.4. Notes on each template

**Összegzés.** The optional third argument filters the terms: `SZUMMA(i=1..n, i, i|n)` is the sum of the divisors of n. Put **every** restriction from the statement into the filter: for "the odd divisors", write `SZUMMA(i=1..n, i, i mod 2 = 1 és i|n)`, not just `i mod 2 = 1`.

**Megszámolás.** `DARAB` takes exactly one condition; join several with `és`.

**Maximum / minimum.** Numbers and texts can be compared. **An empty range is an error** ("MAX over an empty range (1..0)"), so require `n>0` in `Ef:` or use `FELTMAX`.

**Feltételes maximum / minimum.** Note the order: **value first, condition second**, `FELTMAX(i=e..u, f(i), T(i))`. When nothing qualifies, `van` is false and `maxind`/`maxert` have no defined value: any test value passes for them, and every use must be guarded with `van -> …`.

**Keresés.** `(van,ind)=KERES(i=1..n, t[i]<0)` gives the first match. When nothing matches, `van` is false and **`ind` has no defined value**:

- The test case still needs a value for `ind`, and **any valid value passes** (`0`, `n+1`, …). The editor does not compute it.
- In that test case, `ind` then holds the arbitrary test value, so an **unguarded** use such as `s=t[ind]` is checked against it: with `ind: 0` it stops with "index 0 is out of range", and with `ind: 1` it may even pass by accident. Guard every use with `van -> …`, and give such a test an `ind` that would expose a missing guard (`0`).
- Using the pattern's index directly is an error: "`KERES(i=1..n, t[i]>0).2` has no defined value".

**Reverse Keresés** (the last match): run the search over the negated range and negate the index, `(van,-ind)=KERES(i=-n..-1, t[-i]>0)`. It is a trick, so add a comment: `/* the last i with t[i]>0: i runs n, n-1, …, 1 as -i */`.

**Eldöntés / Mind eldöntés.** `van=VAN(i=1..n, t[i]<0)` is the macro form of `van=∃i∈[1..n]:(t[i]<0)`, and `MIND` of `∀`.

**Kiválasztás.** `ind=KIVÁLASZT(i>=1, t[i] mod 2 = 1)`. The precondition must prove a match exists (`∃i∈[1..n]:(t[i] mod 2 = 1)`), and because nothing else in `Uf:` mentions `n`, add `és n=n`. **Write `>=`, not `≥`**: inside KIVÁLASZT the online editor accepts only `>=` (speci-local: "Expected `>=` but found `≥`"). Without a match, evaluation stops, usually with "index … is out of range" (the search runs past the array), otherwise with "SELECT found no matching element".

**Másolás.** `y=MÁSOL(i=1..n, abs(t[i]))`. The result is indexed `e..u`, so the output's declared start must match (`y∈Z[1..n]` for `i=1..n`, `y∈Z[0..]` for `i=0..n-1`). The slides write `y∈H[1..u-e+1]` with `y[i-e+1]`; that is the same when `e=1`.

**Kiválogatás.** `(db,y)=KIVÁLOGAT(i=1..n, t[i]>0)` collects the **indexes**; `(db,y)=KIVÁLOGAT(i=1..n, t[i]>0, t[i])` collects the values (or record fields, or anything else). `y` is indexed from 1: declare it `y∈N[1..db]` (or `y∈Z[]`). The slides' dynamic-array version (`y:=[]`, `y:=Végére(y,f(i))`, short form `(,y)=KIVÁLOGAT(…)` with `y∈H[1..]`) works in the specification, but the Stuki runner has no `Végére`: use the `db` version in structograms.

### 3.5. Reduction (*mintavisszavezetés*)

1. **Pick the template** (3.1).
2. **Write the mapping**: what each abstract part of the template is in this task. `~` reads "corresponds to". Map every part the template has: the interval `e..u`, `T(i)` and/or `f(i)`, and the outputs.

   ```text
   Visszavezetés: Keresés
     e..u ~ 1..n
     T(i) ~ ar[i]<k
     van  ~ van
     ind  ~ ind
   ```

   Keep the template's output names when they fit (`van`, `ind`, `db`), so the mapping is mostly `x ~ x`. When the task needs its own output (`sorszam`, `-1` if none), keep the template's results as auxiliary variables (`Sa:`) and set the output from them in `Uf:` (8 shows this).
3. **The specification** is the template's short form with the mapping substituted: `(van,ind)=KERES(i=1..n, ar[i]<k)`, plus what the task adds (the bounds in `Ef:`, the `-1` in `Uf:`).
4. **The structogram** is the template's algorithm with the same substitution: `i:=1`, `LOOP i≤n és nem (ar[i]<k)`, … (6.1).
5. In the `.sp` file, put the mapping in a `/* … */` comment after `Uf:`, so the file stays a valid specification.

**Several templates.** Reduce each one separately and number them. A template **inside** another (the inner one runs for every `i` of the outer one) becomes a function: its own reduction, an `Fv:` line, and its own structogram diagram.

```text
Visszavezetés 1: Maximumkiválasztás (the largest value of row i)
  e..u   ~ 1..m
  f(j)   ~ t[i,j]
  → Fv: sormax:N->N, sormax(i)=MAX(j=1..m, t[i,j]).2

Visszavezetés 2: Megszámolás (rows whose largest value is above k)
  e..u ~ 1..n
  T(i) ~ sormax(i)>k
```

Use a different loop variable for the inner template (`j` inside `i`), as above.

### 3.6. Using the results

- **Scalar results** are bound with `=`: `db=DARAB(…)`, `s=SZUMMA(…)`, `van=VAN(…)`.
- **Tuple results** are unpacked: `(maxind,maxert)=MAX(…)`, `(van,ind)=KERES(…)`, `(db,y)=KIVÁLOGAT(…)`, `(van,maxind,maxert)=FELTMAX(…)`. The number of slots must match the template.
- **Skipping a slot:** `(,maxert)=MAX(…)` keeps the value, `(maxind,)=MAX(…)` keeps the index, `(van,,maxert)=FELTMAX(…)` skips the index. Prefer keeping every result in an auxiliary variable when the reader would otherwise wonder what was skipped.
- **One part:** `MAX(…).2`, `KERES(…).1`. MAX and MIN parts also have names: `.maxind`/`.maxval` and `.minind`/`.minval`; speci-local also knows `.found`/`.ind` (SEARCH), `.cnt`/`.items` (MIS) and `.found`/`.maxind`/`.maxval` (CONDMAX). The positional form works everywhere.
- **Outputs that only matter sometimes:** state them with implications. When `van` is false, `(van -> lib=pl[ind].nev)` is true without looking at `lib`, so a test may give any value for it (`""`). A sentinel the task demands must be stated, because no template produces one:

  ```text
  (van -> sorszam=ind) és (nem van -> sorszam=-1)
  ```

---

## 4. The specification language

### 4.1. Blocks

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
- **Hungarian and English headers may be mixed**, but keep one style per file.
- **A header starts a line** (indentation allowed): a word, the colon, then a space or a line break. Any other line whose first word is capitalised and followed by `:` is read as a misspelt header. Translations are rejected with a pointer to the real header (`Segéd:` → `Sa:`, `Bemenet:` → `Be:`, `Precondition:` → `Pre:`), and wrong case is rejected too (`in:` → `In:`).
- **Only comments may come before the first block.**

### 4.2. Two layouts

**Layout A**, content on the header's line, suits short English specifications:

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

**Layout B**, the preferred Hungarian one: the header alone on its line, one declaration per line with a comma after each except the last, and a blank line between blocks.

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

### 4.3. Comments

`// …` runs to the end of the line, and `/* … */` may span lines. Both may follow real content. Comment markers inside a text literal (`"//"`) are text.

```text
Be: szam∈N
Ki: osztok∈N
Ef: szam>=1
Uf: osztok=DARAB(i=1..szam, i|szam)    // i|szam: i osztja szam-ot
/* Visszavezetés: Megszámolás
     e..u ~ 1..szam
     T(i) ~ i|szam */
```

```yaml
szam: 12
osztok: 6
```

### 4.4. Names

- A name starts with a letter or `_`, followed by letters, digits or `_`. Accented letters are allowed (`szám`), but plain ASCII is the safer habit. Names are case-sensitive.
- **Variables:** lowercase by convention (`n`, `szamok`, `maxTav`, `x1`). **Types:** start with an uppercase letter. This is enforced for aliases: `Varos=(…)` is an alias, `varos=…` is an error.
- **Reserved words** (operators, so never names): `and és or vagy not nem mod div in eleme exists létezik forall minden true igaz false hamis`. The online editor reports `Aux: exists∈L` as `incomplite type for exists: undefined is not boolean`; speci-local says "`exists` is a reserved word and can't be used as a name".
- Not reserved: `x` (it means "product" only inside types), `element` (it is *not* a synonym of `∈`), `ha`/`if`/`egyébként`/`otherwise` (special only inside `{…}` cases), and every lowercase spelling of a macro (`max`, `sum`, `van`, `db`). A variable may even share a built-in's name (`abs∈Z[1..n]` next to `abs(x)`).

### 4.5. Types and declarations

A declaration is `name∈Type`. `in` and `eleme` are word forms of `∈`; `element` is not. Declarations are separated by commas. A type alias `Name=Type` may stand anywhere in a declaration block, before or after its use, and aliases may chain.

**Elementary sets**

| Set | Meaning | Test-data values |
|:--:|:--|:--|
| `N` | natural numbers 0, 1, 2, … | whole numbers ≥ 0 (a negative one is rejected: "declare it as Z") |
| `Z` | integers | whole numbers |
| `R` | reals | numbers (`1.5`, `3`) |
| `L` | logical | `true`/`false` or `igaz`/`hamis` |
| `S` | text | `abc`, `"abc"`, `""` for empty |
| `C`, `K` | character | a single character |

**Intervals.** `j∈[1..5]` is a whole number between the bounds; a test value outside them is rejected. Bounds may be expressions of earlier inputs: `[2+3..n*2]`.

**Arrays**

| Declaration | Indexes | Test data |
|:--|:--|:--|
| `x∈Z[1..n]` | 1..n | exactly n elements |
| `x∈Z[0..n-1]`, `x∈Z[-1..1]` | as written | exactly that many |
| `x∈Z[1..]` | from 1, length from the data | any length |
| `x∈Z[0..]` | from 0 | any length |
| `x∈Z[]` | same as `Z[1..]` | any length |
| `x∈S[1..8]=["a","b",…]` | constant with initial value | **must not** appear in test data |

`length(x)`/`hossz(x)`, `from(x)`/`tól(x)` and `to(x)`/`ig(x)` give the length, first index and last index of an array. Indexing outside the range is a runtime error ("index 3 is out of range … its indexes are 1..2"). Texts (`S`) can't be indexed.

**Matrices.** `t∈N[1..n,1..m]` is an n×m matrix, indexed as `t[i,j]`. `Z[,]` means `Z[1..,1..]`. Test data gives a list of rows: `[[1,2],[3,4]]`. speci-local stores `Z[1..n,1..m]` and `Z[1..n][1..m]` the same way (an array of rows), so `t[i,j]` and `t[i][j]` both work. Write the comma form for a rectangular grid, and `Z[1..n][]` only when the rows may have different lengths. Every row of `Z[1..n,1..m]` must have exactly m elements.

**Records**

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

Fields are separated by ` x ` (`*` is also accepted); a comma between fields is an error. A field is read as `r.field`. A record type may also be written inline: `r∈(nev:S x kor:N)`. Fields may be arrays or other records: `Diak=(nev:S x jegyek:[1..5][])`.

**Constants.** `x∈T=expr` gives the variable its value in the specification. It is still a declared variable: it must appear in `Uf:`, and it must **not** be repeated in the test data (the online editor says `data cannot contain item with the same name: x`). An initialised matrix keeps the declared index ranges: `m∈Z[-1..1,-1..1]=[[2,1,2],[1,0,1],[2,1,1]]` gives `m[-1,-1]=2`.

### 4.6. Expressions and operators

**Precedence (loosest first)**

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

- `van -> r=ind és nem van -> r=-1` is two implications joined by `és`, as intended. Still, parenthesize them for the reader: `(van -> r=ind) és (nem van -> r=-1)`.
- `a vagy b -> c` means `a vagy (b -> c)`. Parenthesize when mixing.
- `nem a=b` means `nem (a=b)`.

**Logic and comparison**

- `true`/`igaz`, `false`/`hamis`. `∧`, `∨` and `→` are **rejected** ("Unexpected character"); write `and`/`és`, `or`/`vagy`, `->`.
- Equality compares by value. Numbers compare across `Z`/`R` (within 1e-9). Texts compare exactly and order lexicographically with `<`. Records compare field by field. **Arrays are equal only if their index ranges match too:** `[1..2]` vs `[0..1]` is an error, not `false`.
- `x∈[a..b]` tests membership in an interval: `Ef: a∈[2..7]`.

**Arithmetic**

| Operator | Meaning |
|:--|:--|
| `+ - *` | overflow is an error, not a wrap-around |
| `/` | exact quotient: whole if divisible, otherwise real (`3/2 = 1.5`) |
| `div` | whole division rounded down (`-7 div 2 = -4`) |
| `mod`, `%` | remainder, sign follows the dividend (`-7 mod 2 = -1`) |
| `a\|b` | *a divides b* (`i\|n` ⇔ `n mod i = 0`); `a∤b` is its negation |
| `n!` | factorial |

Built-in functions (lowercase only): `abs`, `sqrt` (real result), `round`/`kerekít` (halves round up), `length`/`hossz`, `from`/`tól`, `to`/`ig`. `ABS(x)` is rejected: write `abs(x)`.

**Literals, tuples and cases**

- Numbers: `42`, `-3`, `2.5`. Texts: `"alma"`, `'alma'` or `„alma”`.
- Array literals: `[1, 2, 3]` (indexed from 1), `[0..:5, 6]` (from 0), `[1..,1..:[1,2],[3,4]]` (a matrix with explicit starts).
- Tuples: `(a, b)`. A part is read as `.1`, `.2`, … (1-based): `(3,5).2 = 5`. An empty slot `(a,,c)` is only allowed on the left side of `=`, to skip a template result.
- **Cases** pick a value by condition: the first case whose condition holds gives the value. The commas before `ha` and `egyébként` are optional. If no case holds and there is no `egyébként`, evaluation stops with "None of the cases … holds".

  ```text
  {érték1, ha feltétel1; érték2, ha feltétel2; érték3 egyébként}
  {1, if x>0; 0, if x=0; -1 otherwise}
  ```

**Quantifiers**

```text
∀i∈[1..n]:(t[i]>0)          forall i in [1..n]:(t[i]>0)        minden i eleme [1..n]:(t[i]>0)
∃i∈[1..n]:(t[i] mod 2 = 1)  exists i in [1..n]:(…)             létezik i eleme [1..n]:(…)
```

- The range is always an interval `[lo..hi]`. Over an empty range, `∀` is true and `∃` is false.
- **The body must be in parentheses.** `∀i∈[1..n]:t[i]>0` is parsed as `(∀i∈[1..n]:t[i]) > 0` and fails with "`t[i]` should be true or false".
- Nest them for matrices: `∀i∈[1..n]:(∀j∈[1..m]:(a[i,j]>=0))`.
- The quantified variable is local. It may reuse a name from an enclosing formula, but a fresh name reads better.

### 4.7. Functions (`Fv:` / `Fn:`)

```text
Fv: f:Dom->Cod, f(p)=body
```

- **One function per header.** `Fv: f:…, f(…)=…, g:…, g(…)=…` is rejected ("Only one function per `Fn:` header"); start each function on its own `Fv:` line. Consecutive `Fv:` headers are fine.
- **The signature is compulsory** and comes first. `Fv: d(a)=a*2` alone is rejected ("has no signature"). The name in the definition must match the signature.
- **The arrow is ASCII `->`.** Several parameters go in a product domain: `osszead:Z x Z->Z, osszead(a,b)=a+b`. Array parameters are written `Z[]`.
- The body is one expression, often a template. It sees every declared variable, and parameters shadow them. Functions may call other functions and themselves (recursion depth up to 500).
- **Tuple-valued templates need the whole tuple in the codomain.** `MAX`/`MIN` give 2 values, `KERES` 2, `KIVÁLOGAT` 2 and `FELTMAX`/`FELTMIN` 3. `f:N->N, f(i)=MAX(…)` is rejected ("`MAX` gives 2 values (index, value), but the signature of `f` has 1 in its codomain"). Write `f:N->N x N` and use `f(i).2`, or keep one part in the body: `f(i)=MAX(…).2`.

Examples (each used in a case study):

```text
Fv: maxsor:N->N x N, maxsor(i)=MAX(j=1..m, t[i,j])
Fv: osszeg:N->N, osszeg(i)=SZUMMA(j=i-d..i-1, tr[j])
Fv: mind5felett:N->L, mind5felett(i)=MIND(j=1..4, kave[i,j]>5)
Fv: uj:N->L, uj(i)=MIND(j=1..i-1, futas[i].ido≠futas[j].ido)
Fv: elojel:Z->Z, elojel(x)={1, ha x>0; 0, ha x=0; -1 egyébként}
Fv: fakt:N->N, fakt(i)={1, ha i=0; i*fakt(i-1) egyébként}
```

### 4.8. Preconditions (`Ef:` / `Pre:`)

- **No precondition:** `Ef: -` (an en dash `–` also works). `Ef: igaz` is equivalent.
- **Ranges:** `1<=n<=100`, `n∈[1..100]`, `n>=1 és n<=100`.
- **Element constraints:** `∀i∈[1..n]:(1<=ar[i] és ar[i]<=10000)`.
- **Existence**, needed before `KIVÁLASZT`: `∃i∈[1..n]:(t[i] mod 2 = 1)`.
- **Outputs may not appear in the precondition.** The online editor says `Output data x is in the precondition!`; speci-local says "Output `x` is used in the precondition". Auxiliary variables are allowed.
- **Copy every bound from the statement separately.** A bound that is too loose still evaluates green, so this mistake never shows on the samples.
- A test case whose input violates the precondition is red ("The precondition (Ef) is false: `n>5` does not hold").

### 4.9. Postconditions (`Uf:` / `Post:`)

**The All Variables Mandate.** Every declared variable (inputs, auxiliary variables, outputs and constants) must occur in the postcondition, directly or inside a function the postcondition calls. Occurring only in `Ef:` does not count.

- Online editor: `Line 5, column 1: egyeb is missing from the post condition!` (the position is the **declaration**).
- speci-local: "Input `egyeb` is never used in the postcondition", with the hint to append `and egyeb=egyeb` (`és egyeb=egyeb` reads better in a Hungarian file).

The filler `és x=x` satisfies the check. A bound variable such as `n` in `i=1..n` or `∀i∈[1..n]` counts as used.

**Template style and relational style.** A postcondition can be any first-order formula. The lecture notes sometimes use the relational style, which needs no auxiliary variables:

```text
Uf: ∃i∈[1..n]:(∀j∈[1..n]:(tav[j]<tav[i] vagy (tav[j]=tav[i] és ar[j]>=ar[i])) és ar[i]=max)
```

Bind outputs **through the witness** (`ar[i]=max`, `sorszam=i`), and never index with a value that may be a sentinel (`ar[sorszam]` when `sorszam=-1` is an out-of-range read).

| Use… | When |
|:--|:--|
| template style (`MAX`, `KERES`, `KIVÁLOGAT`, …) | the default, and always in a full exercise: it maps onto the algorithm |
| relational style (`∃`/`∀`) | the output is defined by a property, or the task asks for the relational model |

---

## 5. Test cases

### 5.1. Format

Each test case is YAML: one `name: value` line per variable, with a `#` comment saying what the test checks.

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

- **Every declared variable** (inputs, auxiliary variables and outputs) needs a line, even when its value doesn't matter, like `ind` after a failed search ("Auxiliary variable `ind` is missing from this test case").
- **Constants with an initial value must not** have a line ("`lapok` already has its value in the specification"; online: `data cannot contain item with the same name: lapok`).
- **No other keys:** an undeclared or misspelt key is an error ("`Ar` is not declared … Did you mean `ar`").
- **Types are checked:** a negative number for `N`, a value outside an interval, a text for `Z`, an unknown or missing record field, or a wrong array length ("`ar` has 2 element(s), but it is declared as N[1..n] and n = 3, so it needs exactly 3").
- Arrays take their first index from the declaration: for `t∈Z[0..]`, `[5, 6]` means `t[0]=5`.
- Use spaces, never tabs ("found a tab"). In speci-local, Tab inserts spaces and Enter indents like a JSON editor.

### 5.2. How a test case is judged

1. The test data is read and checked against the declarations (5.1). A data error makes the test red.
2. The precondition is evaluated. If it is false, the test is red.
3. The postcondition is evaluated with **all** values from the test case. If it is true, the test is green; otherwise it is red. With every variable given, `=` is a comparison.
4. When a test is red, speci-local re-evaluates the postcondition without the expected outputs to show what the specification gives ("`db`: the test case says 2, the specification gives 3"). An output with no value is set by the first equation that defines it (`db=…`, `(van,ind)=KERES(…)`, or a witness inside `∃`), reading left to right, so keep the defining equation before other uses of the output. If even that fails, it reports that no output values satisfy the postcondition: the formula itself is probably wrong.

Runtime errors stop the test: index out of range, division by zero, a value of the wrong kind ("`+` needs numbers"), MAX over an empty range, KIVÁLASZT without a match, use of an undefined search index, no matching case, recursion that is too deep, or an evaluation over 50 million steps. The message points at the line and column.

A test case passes **only if the expected values are right**: a wrong hand-computed value makes a correct specification look wrong, and a test that agrees with a wrong specification hides the mistake. Work every expected value out from the statement, not from the formula.

### 5.3. Choosing the three tests

Each test must be green for the specification **and** for the structogram. Cover:

1. **A normal case**, ideally the sample from the statement.
2. **The empty or "none" case**: nothing qualifies (`van: false`, a `-1` sentinel, `db: 0` with `y: []`), or the smallest allowed input (`n: 0` or `n: 1`).
3. **A separating case**: one that a plausible wrong solution gets wrong. First vs. best match, a tie (the first maximum wins), a match at the first or last index, the boundary of a condition (`<` vs. `<=`), or a restriction the formula could forget.

For a "none" case, give don't-care outputs a value that would expose an unguarded use (`ind: 0`, `lib: ""`), and say so in the comment.

---

## 6. Structograms

### 6.1. From the template to the structogram

1. **The template's algorithm, substituted.** Take the outline from 3.3 and replace `e`, `u`, `T(i)`, `f(i)` and the output names by the mapping. Keep every element: `ELSE -` branches stay, even when empty.
2. **Then the rest of `Uf:`**, in order, as statements: an implication pair `(van -> sorszam=ind) és (nem van -> sorszam=-1)` becomes `IF van` with `sorszam:=ind` and `ELSE` with `sorszam:=-1`; an equation `osszeg=a+b` becomes `osszeg:=a+b`.
3. **Several templates** follow each other in the order of `Uf:`. A template inside another is a function: draw it as its own diagram (6.3).
4. **Inputs and outputs:** start with a `Be: n, k, ar` statement and end with `Ki: sorszam` (or `In:`/`Out:`). The runner skips these; they show the reader what comes in and goes out.
5. **Local variables** (the loop variable `i`, the template's results that are not declared in the specification) go in the diagram's variable list: `i:Egész`. Types: `Egész`, `Valós`, `Logikai`, `Szöveg`, `Karakter`, `Tömb`, `Rekord`.

speci-local's Stuki tab has a **From spec** button that builds the structogram from the specification with these same algorithms; use it to check a hand-written one.

### 6.2. What the runner accepts

**Run tests** runs the structograms on the specification's test cases. The inputs (and constants) come from the test case; the structogram must give every output and auxiliary variable its value, and then the postcondition is checked on them. A don't-care output may stay unset (`ind` when nothing was found).

- A **statement** is an assignment `x:=expr` (also `t[i]:=…`, `r.mezo:=…`, `(a,b):=f(i)`), `-` (or empty) for "do nothing", or a display-only `Be: …`/`Ki: …` line. One assignment per statement: `a:=1; b:=2` is an error, so draw two boxes. Anything else ("count the positives") is an error.
- A **condition** (decision, loop, case) is a logical expression in the specification language: `i≤n és nem (ar[i]<k)`. Use `és`/`vagy`/`nem` in a Hungarian file, `and`/`or`/`not` in an English one.
- A **pre-test loop** whose text is `i=1..n` is a **count loop**; any other text is the condition it runs while.
- A **post-test loop** runs its body, then **repeats while its condition is true** (like C#'s `do … while`).
- A **multi-way decision** runs the first case whose condition holds. If none holds and there is no default case, the run stops.
- **Output arrays grow** when assigned past their end, so `y[db]:=t[i]` builds a Kiválogatás result without initialisation.
- A variable must have a value before it is read ("`db` has no value at this point"): initialise counters and sums.
- The run stops after 2 million steps ("The structogram doesn't stop").

### 6.3. Functions

A specification function may be used in the structogram directly (`kisebb(i)`): a function with no diagram of its own is evaluated from its `Fv:` definition. When the function's body is a template (an algorithm), draw it as its own diagram:

- **Name** it like its header: `sormax(i:Egész):Egész`, `legnagyobb(k:Egész):Egész x Egész`. A diagram whose name has parentheses is a function; the **first diagram without parentheses is the main program**.
- Its **return value** is the diagram's return text (`returnText`, with `showReturnText: true`): an expression such as `maxert` or a tuple `(maxind, maxert)`.
- Its locals (`j:Egész`, `maxert:Egész`) go in its variable list.
- Call it like the specification does: `IF sormax(i)>k`, or unpack a tuple with `(mi, me):=legnagyobb(n)`.

A one-line function (`kisebb(i)=ar[i]<k`) can be drawn as a diagram whose body is `-` and whose return text is `ar[i]<k`, or left to the specification.

### 6.4. The `stuki` block

In a `.sp` file the structograms follow the test cases as the web Stuki's JSON (the same as a `.st` file), between `/*--- stuki ---` and `---*/`:

- `nodes` is a flat map from id to node. Every node has `id`, `parentId` (`null` for a diagram's root), `hidden: false` and `type`.
- `sequence` nodes hold `childrenIds` in order. Every diagram root and every branch or loop body is a sequence. An empty branch holds one `noop` node.
- `statement` has `text`. `if` has `condition`, `trueBranchSequenceId`, `falseBranchSequenceId`. `while` (pre-test and count loop) and `repeat` (post-test loop) have `condition` and `bodySequenceId`. `switch` has `cases`, each `{"id", "type": "case", "condition", "bodySequenceId"}`, with an optional last `{"id", "type": "default", "bodySequenceId"}`.
- `structograms` lists the diagrams: `name`, `rootId`, `variables`, `returnText`, `showHeader` (true for functions), `showVariables`, `showReturnText`.
- **Texts are HTML: write `<` as `&lt;`, `>` as `&gt;` and `&` as `&amp;`.** `≤ ≥ ≠` need no escaping, so prefer them in conditions.
- Ids may be any unique strings; short readable ones (`w1`, `w1b`) help when writing by hand. The app renumbers them when it saves.

Outline → JSON: each outline line is a node; an indented body is a `sequence` whose parent is the loop, decision or case; `ELSE -` is a sequence holding one `noop`. Section 8 has a complete block.

### 6.5. Drawing it in the answer

Show the structogram as a box drawing, with each box's text exactly as in the JSON. A pre-test loop has its text on top and its body indented below; a decision has its condition in the triangle, the true branch (`T`) on the left and the false branch (`F`) on the right:

```text
┌──────────────────────────────────┐
│ i:=1                             │
├──────────────────────────────────┤
│ i≤n és nem (ar[i]<k)             │
│   ┌──────────────────────────────┤
│   │ i:=i+1                       │
├───┴──────────────────────────────┤
│ \             van              / │
│  \ T                        F /  │
├────────────────┬─────────────────┤
│ ind:=i         │ -               │
└────────────────┴─────────────────┘
```

A post-test loop has its body indented on top and its condition at the bottom. A multi-way decision has one column per case: its condition on top, its body below. Write the local variables next to the drawing (`Változó i:Egész`).

---

## 7. C#

The program translates the structogram **one element to one statement**, so the reader can put the two side by side.

### 7.1. Shape of the program

- A console program: `using System;`, `class Program`, `static void Main()`. Inside `Main`, three commented parts: `// Be:` (reading the input), the algorithm (`// (van,ind)=KERES(i=1..n, ar[i]<k)`, one comment per template), `// Ki:` (printing the output).
- **The specification's names**, unchanged (ASCII spelling: `maxert` for `maxért`), and explicit types (`int`, `bool`) rather than `var`.
- **No shortcuts:** no LINQ, no `Array.Sort`/`Max`, no `?:` chains, no `break`. Braces around every body, one statement per line.
- **Indexes as in the specification:** declare `ar∈N[1..n]` as `int[] ar = new int[n + 1];` and leave index 0 unused, so `ar[i]` means the same in all three parts.
- **A specification with functions:** declare its variables as `static` fields of `Program` (`static int n, k;`, `static int[] ar = new int[0];`) so that the methods see them, as functions see every variable in the specification. Without functions, keep them local in `Main`.
- **C# wants a value before every read:** give a template's possibly-undefined results a starting value with a comment (`int ind = 0; // csak akkor értelmes, ha van`).

### 7.2. Translation table

| Specification / structogram | C# |
|:--|:--|
| `N`, `Z` | `int` (`long` if values may exceed 2·10⁹) |
| `R` | `double` |
| `L` | `bool` |
| `S` | `string` |
| `C`, `K` | `char` |
| `t∈Z[1..n]` | `int[] t = new int[n + 1];` |
| `t∈Z[0..n-1]` | `int[] t = new int[n];` |
| `t∈Z[1..n,1..m]` | `int[,] t = new int[n + 1, m + 1];`, read as `t[i, j]` |
| `Varos=(nev:S x tav:N)` | `struct Varos { public string nev; public int tav; }` |
| `y∈Z[1..db]` (Kiválogatás) | `int[] y = new int[n + 1];` (room for every element), only `y[1..db]` is used |
| `Fv: kisebb:N->L` | `static bool kisebb(int i) { return ar[i] < k; }` |
| `Fv: f:N->N x N` | `static (int, int) f(int i)`, used as `(mi, me) = f(n);` |
| `x:=e` | `x = e;` |
| count loop `i=e..u` | `for (int i = e; i <= u; i++) { … }` |
| pre-test loop `c` | `while (c) { … }` |
| post-test loop `c` | `do { … } while (c);` |
| decision `c` | `if (c) { … } else { … }`; leave out an `else` whose only content is `-` |
| multi-way decision | `if (c1) { … } else if (c2) { … } else { … }` |
| `és`, `vagy`, `nem` | `&&`, `\|\|`, `!` |
| `=`, `≠`, `≤`, `≥` | `==`, `!=`, `<=`, `>=` |
| `igaz`, `hamis` | `true`, `false` |
| `a mod b`, `a div b` | `a % b`, `a / b` (the same for non-negative numbers; for negative ones `div` rounds down but C# `/` rounds toward zero) |
| `a\|b` (divides) | `b % a == 0` |
| `abs(x)`, `sqrt(x)` | `Math.Abs(x)`, `Math.Sqrt(x)` |

### 7.3. Input and output

- **Follow the statement's format** when it gives one (Mester style: read standard input, no prompts, print only the result). Otherwise read the inputs in declaration order, one line per scalar and one space-separated line per array, and print each output on its own line.
- Split lines with `Console.ReadLine()!.Split(' ', StringSplitOptions.RemoveEmptyEntries)` (the `!` tells the compiler the line exists).
- Read reals with `double.Parse(s, CultureInfo.InvariantCulture)` (`using System.Globalization;`): on a Hungarian system, plain `double.Parse("1.5")` fails, because the decimal separator there is a comma.
- `Console.WriteLine(van)` prints `True`/`False`; print what the statement asks for instead (`if (van) { Console.WriteLine("igen"); } else { … }`).

---

## 8. A complete exercise

> *N trees with prices. Give the 1-based index of a tree cheaper than K forints; the smallest such index; −1 if none. 1 ≤ N ≤ 100, 1 ≤ K ≤ 10 000, 1 ≤ price ≤ 10 000. Input: N and K on the first line, the N prices on the second. Output: the index.*

**Visszavezetés.** We need the first tree that is cheaper than K, and there may be none: that is Keresés. Its result `van` tells whether there is one, `ind` which one; the task's output `sorszam` is `ind`, or −1 when there is none.

```text
Visszavezetés: Keresés
  e..u ~ 1..n
  T(i) ~ ar[i]<k
  van  ~ van
  ind  ~ ind
```

**`karacsonyfak.sp`**: the specification, the three tests and the structogram. All three tests are green on the Specification tab and in the Stuki tab's Run tests.

```text
// title: Karácsonyfák
Be:
n∈N,
k∈N,
ar∈N[1..n]

Sa:
van∈L,
ind∈N

Ki:
sorszam∈Z

Ef:
1<=n<=100 és 1<=k<=10000 és ∀i∈[1..n]:(1<=ar[i] és ar[i]<=10000)

Uf:
(van,ind)=KERES(i=1..n, ar[i]<k) és
(van -> sorszam=ind) és
(nem van -> sorszam=-1)

/* Visszavezetés: Keresés
     e..u  ~ 1..n
     T(i)  ~ ar[i]<k
     van   ~ van
     ind   ~ ind */

/*--- test ---
# van ilyen fa: a 3. az első, ami olcsóbb K-nál
n: 6
k: 2000
ar: [2500, 5300, 1900, 2400, 8800, 5300]
van: true
ind: 3
sorszam: 3
---*/

/*--- test ---
# nincs ilyen fa: ind bármi lehet, sorszam -1
n: 3
k: 100
ar: [2500, 5300, 1900]
van: false
ind: 0
sorszam: -1
---*/

/*--- test ---
# a legkisebb sorszám kell, nem a legolcsóbb fa
n: 4
k: 5000
ar: [6000, 3000, 1000, 200]
van: true
ind: 2
sorszam: 2
---*/

/*--- stuki ---
{
  "nodes": {
    "root": {"id": "root", "parentId": null, "hidden": false, "type": "sequence",
             "childrenIds": ["s1", "s2", "w1", "s4", "if1", "s7"]},
    "s1": {"id": "s1", "parentId": "root", "hidden": false, "type": "statement", "text": "Be: n, k, ar"},
    "s2": {"id": "s2", "parentId": "root", "hidden": false, "type": "statement", "text": "i:=1"},
    "w1": {"id": "w1", "parentId": "root", "hidden": false, "type": "while",
           "condition": "i≤n és nem (ar[i]&lt;k)", "bodySequenceId": "w1b"},
    "w1b": {"id": "w1b", "parentId": "w1", "hidden": false, "type": "sequence", "childrenIds": ["s3"]},
    "s3": {"id": "s3", "parentId": "w1b", "hidden": false, "type": "statement", "text": "i:=i+1"},
    "s4": {"id": "s4", "parentId": "root", "hidden": false, "type": "statement", "text": "van:=i≤n"},
    "if1": {"id": "if1", "parentId": "root", "hidden": false, "type": "if", "condition": "van",
            "trueBranchSequenceId": "if1t", "falseBranchSequenceId": "if1f"},
    "if1t": {"id": "if1t", "parentId": "if1", "hidden": false, "type": "sequence", "childrenIds": ["s5", "s6"]},
    "s5": {"id": "s5", "parentId": "if1t", "hidden": false, "type": "statement", "text": "ind:=i"},
    "s6": {"id": "s6", "parentId": "if1t", "hidden": false, "type": "statement", "text": "sorszam:=ind"},
    "if1f": {"id": "if1f", "parentId": "if1", "hidden": false, "type": "sequence", "childrenIds": ["s8"]},
    "s8": {"id": "s8", "parentId": "if1f", "hidden": false, "type": "statement", "text": "sorszam:=-1"},
    "s7": {"id": "s7", "parentId": "root", "hidden": false, "type": "statement", "text": "Ki: sorszam"}
  },
  "structograms": [
    {"name": "Karácsonyfák", "rootId": "root", "variables": "i:Egész", "returnText": "",
     "showHeader": false, "showVariables": true, "showReturnText": false}
  ]
}
---*/
```

The structogram. The Keresés template, then the `-1` from `Uf:` (Változó `i:Egész`):

```text
┌──────────────────────────────────┐
│ Be: n, k, ar                     │
├──────────────────────────────────┤
│ i:=1                             │
├──────────────────────────────────┤
│ i≤n és nem (ar[i]<k)             │
│   ┌──────────────────────────────┤
│   │ i:=i+1                       │
├───┴──────────────────────────────┤
│ van:=i≤n                         │
├──────────────────────────────────┤
│ \             van              / │
│  \ T                        F /  │
├────────────────┬─────────────────┤
│ ind:=i         │ sorszam:=-1     │
├────────────────┤                 │
│ sorszam:=ind   │                 │
├────────────────┴─────────────────┤
│ Ki: sorszam                      │
└──────────────────────────────────┘
```

**`karacsonyfak.cs`**. For the three tests it prints `3`, `-1` and `2`.

```csharp
using System;

class Program
{
    static void Main()
    {
        // Be: n, k, ar
        string[] sor = Console.ReadLine()!.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        int n = int.Parse(sor[0]);
        int k = int.Parse(sor[1]);
        int[] ar = new int[n + 1]; // ar[1..n], mint a specifikációban
        sor = Console.ReadLine()!.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        for (int j = 1; j <= n; j++)
        {
            ar[j] = int.Parse(sor[j - 1]);
        }

        // (van,ind)=KERES(i=1..n, ar[i]<k)
        int i = 1;
        while (i <= n && !(ar[i] < k))
        {
            i = i + 1;
        }
        bool van = i <= n;
        int ind = 0; // csak akkor értelmes, ha van
        int sorszam;
        if (van)
        {
            ind = i;
            sorszam = ind;
        }
        else
        {
            sorszam = -1;
        }

        // Ki: sorszam
        Console.WriteLine(sorszam);
    }
}
```

---

## 9. Case studies

All of these are green in speci-local. They come from the course materials, so most use English headers and names; when answering, apply the Hungarian default unless the user writes in English.

### Task 1: The Discworld libraries (Keresés in records, implication)

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

Two Maximumkiválasztás, one inside the other: `maxcol` returns the whole `(index, value)` tuple, so its codomain is `N x N`. The row maximums are 12, 23, 4, 23, 4 and 1; the first 23 is in row 2, and that row's maximum is in column 3.

### Task 3: Trurl's robots (Kiválogatás with a window sum)

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

### Task 4: Catbert's coffee breaks (Keresés with a nested Mind eldöntés)

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

### Task 5: Dogbert's star (Feltételes maximumkeresés, skipped slot)

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

### Task 6: Travel agency (relational style, Hungarian)

> *An agency sells tickets to cities at given distances and prices. Give the price of the ticket to the farthest city; if several are equally far, the cheapest. 1 ≤ N ≤ 100, distance 1 ≤ T ≤ 20 000, price 1 ≤ Á ≤ 2 000 000.*

```text
Be:
n∈N,
ar∈N[1..n],
tav∈N[1..n]

Ki:
max∈N

Ef:
1<=n és n<=100 és ∀i∈[1..n]:(1<=ar[i] és ar[i]<=2000000 és 1<=tav[i] és tav[i]<=20000)

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

There is a city `i` such that every city is nearer, or equally far but not cheaper, and the output is its price. The output may be called `max`, because the macro is the uppercase `MAX`. The template-style equivalent (Maximumkiválasztás, Kiválogatás, Minimumkiválasztás), with records:

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

Auxiliary variables have to be in the test case too, which makes the relational form shorter here; the template form is the one that gives a structogram.

### Task 7: Number of divisors (Megszámolás)

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
# a prime: 1 and itself
szam: 13
osztok: 2
```

### Task 8: Sum of the odd divisors (filtered Összegzés)

```text
Be: n∈N
Ki: osszeg∈N
Ef: n>=1
Uf: osszeg=SZUMMA(i=1..n, i, i mod 2 = 1 és i|n)
```

```yaml
# 1 + 5
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

### Task 10: Christmas trees, relational style

The task of section 8, without auxiliary variables:

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

`∀j∈[1..i-1]:(ar[j]>=k)` makes the witness the **first** match, and `sorszam` is `Z` because `-1` is not in `N`.

### Task 11: Distinct times (backward Mind eldöntés in a function)

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

### Task 12: Long runs (three-argument Kiválogatás, record field)

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

### Task 13: Sessions where everyone was late (column Mind eldöntés, two-argument Kiválogatás)

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

The two-argument Kiválogatás collects the column **indexes**.

### Task 14: First odd number (Kiválasztás)

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

### Task 15: Profit, when there is one (implication around a Feltételes maximumkeresés)

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

## 10. Pitfalls and error messages

### 10.1. In the specification

| # | Symptom / message | Cause and fix |
|:--:|:--|:--|
| 1 | "Missing space after `Ef:`" | `Ef:-` → `Ef: -`, or put the content on the next line. |
| 2 | "Unknown block header `Segéd:`", online `Expected ",", "Aux:", "Ki:", "Out:", "Sa:", …` | Invented or translated header. The real set is `Be Sa Ki Fv Ef Uf` / `In Aux Out Fn Def Pre Post`. |
| 3 | "Unknown block header `in:`" (case-sensitive) | Write `In:`. |
| 4 | "`Ki:` is in the wrong place" | Order: declarations → `Fv` → `Ef` → `Uf`. |
| 5 | "Missing `Ef:` block" | Add `Ef: -`. |
| 6 | "Second `Be:` block" | Merge the blocks with commas. |
| 7 | "Trailing comma" | No comma after a block's last declaration. |
| 8 | "Expected `,` between declarations" ("Put a comma at the end of the previous declaration") | Layout B needs a comma at each line end except the last. |
| 9 | "`exists` is a reserved word", online `incomplite type for exists: undefined is not boolean` | Rename the variable to `van`/`talalt` in the declaration, the formula **and** the test data. The same goes for `forall`, `in`, `not`, `mod`, `div`, `igaz`, … |
| 10 | "Unexpected character `→`" (or `∧`, `∨`) | Write `->`, `and`/`és`, `or`/`vagy`. |
| 11 | "Expected `>=` but found `≥`" | Inside KIVÁLASZT/SELECT only `>=` works. |
| 12 | "Only one function per `Fn:` header" | One function per `Fv:` line; repeat the header. |
| 13 | "`f` has no signature" | Write `f:N->N, f(i)=…`. |
| 14 | "`MAX` gives 2 values (index, value), but the signature of `f` has 1 in its codomain" | Codomain `N x N` and use `f(i).2`, or `f(i)=MAX(…).2`. |
| 15 | "Unknown function `ABS`" / "Unknown function `sum`" | Built-ins are lowercase (`abs`); macros are uppercase (`SUM`). |
| 16 | "Expected `∈` after `n` … but found `element`" | Declarations are `x∈T` (or `x in T`, `x eleme T`). `element` is not a keyword. |
| 17 | "Expected an operator, but found `:=`" | `:=` exists only in structograms; use `=`. |
| 18 | "Record fields are separated by ` x `, not commas" | `(nev:S x kor:N)`. |
| 19 | "Input `x` is never used in the postcondition", online `x is missing from the post condition!` | Use it in `Uf:` or append `és x=x`. This applies to aux, outputs and constants too. |
| 20 | "Output `x` is used in the precondition", online `Output data x is in the precondition!` | Move that condition to `Uf:`. |
| 21 | "`x` is not declared … Did you mean" | Typo, or a missing declaration. |
| 22 | "DARAB got 2 argument(s) after the range, but needs 1" | Wrong argument count (3.2); join conditions with `és`. |
| 23 | "SUM got 3 argument(s) after the range, but needs 1 or 2" | The general summation `SZUMMA(…, kezd, add)` is not supported (3.3). |
| 24 | Mixed languages (`Be:` over `nums`, `cnt`) | Not an error, but keep Hungarian headers with Hungarian names, or English with English. |

### 10.2. In a test case

| # | Symptom / message | Cause and fix |
|:--:|:--|:--|
| 1 | "Auxiliary variable `ind` is missing from this test case" | Every non-constant variable needs a value; any valid value for a don't-care. |
| 2 | "`lapok` already has its value in the specification", online `data cannot contain item with the same name: lapok` | Delete constants from the test data. |
| 3 | "`sorszam` is -1, but it is declared as N" | Declare it `Z`. |
| 4 | "`ar` has 2 element(s), but it is declared as N[1..n] and n = 3, so it needs exactly 3" | The array length must match the declared range. |
| 5 | "`T` is not declared in the specification … Did you mean `t`?" | Test keys match the declared names exactly. |
| 6 | "found a tab" | Indent YAML with spaces. |
| 7 | "The precondition (Ef) is false: … does not hold" | The input breaks a bound; fix the test, or the bound if it is wrong. |
| 8 | "The postcondition (Uf) is false: … does not hold" | Either the expected value is wrong (recompute it by hand) or the formula is; speci-local shows what the specification gives. |
| 9 | "`t[i]` should be true or false" after a quantifier | Parenthesize the body: `∀i∈[1..n]:(t[i]>0)`. |
| 10 | "index 6 is out of range" | A loop that reads `t[i+1]` must stop at `n-1`; `0..`-based arrays loop over `0..length(t)-1` (or `from(t)..to(t)`). After a failed search, an unguarded `t[ind]` reads the test's arbitrary `ind`. |
| 11 | "MAX over an empty range (1..0)" | Require `n>0` in `Ef:`, or use `FELTMAX`. |
| 12 | "`KERES(…).2` has no defined value" | A failed search's index was used. Guard it with `van -> …`. |
| 13 | "SELECT found no matching element" | KIVÁLASZT needs `∃…` in `Ef:`; if a match may be missing, use KERES. |
| 14 | "The two arrays don't have the same index range" | For `y=MÁSOL(i=0..n-1,…)`, declare `y∈Z[0..]`; Kiválogatás results start at 1. |
| 15 | "None of the cases of `{…}` holds" | Add an `egyébként` case. |
| 16 | A green test whose specification is still wrong | The filter is missing a restriction (Task 8), a bound is too loose (4.8), the first match is not enforced (Task 10), or an unguarded index passes by accident (3.4). Add tests that separate the wrong reading from the right one. |

### 10.3. In a structogram (Run tests)

| # | Message | Cause and fix |
|:--:|:--|:--|
| 1 | "`db` has no value at this point" | A variable is read before it gets a value: initialise it (`db:=0`), or check the order of the boxes. |
| 2 | "`…` is not a step: a statement must give a variable its value, like `x:=…`" | A statement holds a condition or prose. Statements are assignments. |
| 3 | "`…` can't be run: …" | The text is not an expression: prose, two assignments joined by `;`, `←` instead of `:=`. |
| 4 | "`…` is not a condition" | An assignment in a decision or loop. |
| 5 | "`f` is not a function of the specification or a structogram" | The call's name doesn't match a diagram name (`f(x:Egész):…`) or an `Fv:` function. |
| 6 | "`f` takes 1 argument(s) but gets 2 here" | The diagram header and the call disagree. |
| 7 | "The structogram of `f` has no return value" | Fill in the return text and set `showReturnText`. |
| 8 | "The structogram doesn't stop: after 2 million steps it is still running" | A loop whose variable never changes (`i:=i+1` missing, or in the wrong branch). |
| 9 | "None of the cases of the multi-way decision holds" | The cases don't cover everything: add the missing case or a default. |
| 10 | "`sorszam` never gets a value (the specification gives 3)" | An output the structogram forgot to set. |
| 11 | "The postcondition (Uf) is false: …" | The algorithm computes something else: compare it with the template, box by box. |
| 12 | "There is no structogram to run" | The Stuki block is missing or empty. |

---

## 11. The tools

### 11.1. The online editor (progalap.elte.hu/specifikacio)

Requires an ELTE (INF / Caesar) login. Panes: Specification, Data (test cases, one tab per case, `+`/`-` to add or remove), Visualization (Stepper and Step list), and a history sidebar. A symbol palette inserts `∈ ∀ ∃ ≠ ≤ ≥`. In the Data pane, `Shift+Space` then `Tab` scaffolds the declared variable names. Evaluating colours each test tab green or red. Saving gives a permalink; share links carry the whole project as `…/specifikacio/v1/?data=…`. Stuki (progalap.elte.hu/stuki) saves structograms as `.st` files.

### 11.2. speci-local (offline)

A native app for Windows, Linux, macOS and Android. It needs no login and no network.

- **Evaluation** runs in the background as you type. Untick **Auto** to evaluate only on **Evaluate** (Ctrl+Enter).
- **When a test fails**, it shows what the specification would give: "`db`: the test case says 2, the specification gives 3". An error tints its line, and each error card links to the documentation at <https://fcsab.web.elte.hu/en/>.
- **Scaffold** inserts a `name: ` line for every variable missing from the test case. It skips constants, and pre-fills `[]` for arrays and `{field: }` for records.
- **Stepper** (Evaluation pane) replays one test case reduction by reduction. The part being evaluated is highlighted, the loop variables are listed, and `↓`/`↑` mark entering and leaving your functions. Step with ←/→. *Step list* shows every step.
- **Editor:** completion for headers, set names after `∈`, declared names, and template snippets (`SUM` → `SUM(i=1..n, |)`). Alt+1…Alt+6 insert `∈ ∀ ∃ ≠ ≤ ≥`. Typing `forall`, `exists` or `leq` and completing turns them into symbols. `<=`, `>=` and `!=` typed by hand become `≤ ≥ ≠`, so type SELECT's `>=` through the `SELECT` completion snippet.
- **`.sp` files** hold the whole project: the specification as plain text, an optional `// title: …` first line, each test case in a `/*--- test --- … ---*/` comment, and the structograms (if the Stuki tab was used) in a `/*--- stuki --- {JSON} ---*/` comment. The whole file is still a valid specification and can be pasted into the online editor as-is. Section 8 is a complete example.
- **Stuki tab:** draw structograms (saved in the same `.sp` file; a web Stuki `.st` file can be opened into the project), **From spec** builds them from the specification, and **Run tests** runs them on the same test cases (6.2). Copying gives a PNG picture of a structogram.
- **Sharing:** Ctrl+Shift+C copies the whole project (structograms included) as one `SPECI1:…` line, and Ctrl+Shift+V pastes it back. Paste also accepts plain specification text, `.st` text and the web editors' `?data=` links. *Open in web editor* and *Copy web editor link* hand the project to the online editor.

---

## 12. Grammar

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

(* the texts of structogram elements *)
statement   = "" | "-" | target ":=" expr
            | ( "Be:" | "In:" | "Ki:" | "Out:" ) … ;   (* display only *)
target      = name { "[" expr { "," expr } "]" | "." name }
            | "(" [ target ] { "," [ target ] } ")" ;
loophead    = name "=" sum ".." sum                   (* count loop *)
            | expr ;                                  (* pre-test loop condition *)
```

`MACRO` is one of the names in 3.2. Only SELECT/KIVÁLASZT uses the `>=` form, and only with `>=`. The `x∈[a..b]` membership form allows no chained comparison after it.

---

## 13. Checklist

**Specification**

1. **Headers:** exact spelling, a space or line break after the colon, Hungarian by default, one language per file.
2. **Order:** `Be`/`Sa`/`Ki` → `Fv` (one per header) → `Ef` → `Uf`. `Ef: -` if empty. No trailing comma.
3. **Names:** telling, ASCII, Hungarian; no reserved words (`exists`, `in`, `nem`, …); types capitalised.
4. **Types:** `Z` for anything that can be negative (sentinels), interval types where the statement bounds a value, `C`/`K` for single characters, `[1..n]` arrays matching the input format.
5. **Operators:** `->` not `→`, `abs` not `ABS`, `KIVÁLASZT(i>=…)` not `≥`, quantifier bodies in parentheses, implications parenthesized when joined with `és`/`vagy`.
6. **Templates:** the right one for the question (3.1); argument order (`FELTMAX(i=…, value, condition)`); slot counts (`(van,,maxert)`); Kiválogatás gives indexes unless a third argument is given; MAX needs a non-empty range; every use of a search's index guarded by `van -> …`; sentinel values stated in `Uf:`.
7. **Functions:** signature first with `->`; the whole tuple in the codomain for MAX/KERES/KIVÁLOGAT/FELTMAX bodies.
8. **Mandate:** every declared variable (aux, outputs and constants too) occurs in `Uf:`; no outputs in `Ef:`.
9. **Bounds:** each `Ef:` bound copied from the statement and re-read; loops reading `t[i+1]` stop at `n-1`.

**Test cases**

10. Every non-constant variable present (aux included), no constants, array lengths matching, `true`/`false` for `L`, spaces not tabs.
11. A normal case, a "none"/empty case and a separating case (5.3), each with a `#` comment, every expected value worked out by hand.

**Full exercise**

12. **Reduction:** the template named, every part mapped (`e..u`, `T(i)`/`f(i)`, outputs), one or two sentences saying why it fits.
13. **Structogram:** the template's algorithm box by box with the mapping substituted, then the rest of `Uf:`; `Be:`/`Ki:` boxes; locals listed; `<` escaped as `&lt;` in the JSON; green on the same 3 tests.
14. **C#:** the structogram line by line; the specification's names and indexes; braces everywhere; input and output as the statement says.
15. **Clarity:** could a classmate follow it without asking? If a line needs explaining, add the comment or split the line.
