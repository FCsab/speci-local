//! Case studies from SKILL.md / the ELTE manual, plus error-message checks.

use speci_local::{parse, run_test};

fn render(d: &[speci_local::diag::Diagnostic], spec: &str, data: &str) -> String {
    d.iter()
        .map(|d| d.render(spec, data))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn green(spec: &str, tests: &[&str]) {
    let s = parse(spec).unwrap_or_else(|d| panic!("spec error:\n{}", render(&d, spec, "")));
    for t in tests {
        let o = run_test(&s, t);
        assert!(
            o.pass,
            "expected green for\n{t}\ngot:\n{}",
            render(&o.diags, spec, t)
        );
    }
}

/// Run a test that must fail; return the rendered message.
fn red(spec: &str, test: &str) -> String {
    let s = parse(spec).unwrap_or_else(|d| panic!("spec error:\n{}", render(&d, spec, "")));
    let o = run_test(&s, test);
    assert!(!o.pass, "expected red for\n{test}");
    render(&o.diags, spec, test)
}

fn spec_err(spec: &str) -> String {
    match parse(spec) {
        Ok(_) => panic!("expected a specification error"),
        Err(d) => render(&d, spec, ""),
    }
}

#[test]
fn baseline1_select() {
    green(
        "In: n∈N, nums∈Z[1..n]\nOut: ind∈N\nPre: ∃i∈[1..n]:(nums[i] mod 2 = 1)\nPost: ind=SELECT(i≥1, nums[i] mod 2 = 1) and n=n",
        &[
            "n: 5\nnums: [23, 25, 26, 28, 29]\nind: 1",
            "n: 3\nnums: [2, 4, 7]\n",
        ],
    );
}

#[test]
fn baseline2_count() {
    let spec =
        "In: n∈N, nums∈N[1..n]\nOut: cnt∈N\nPre: -\nPost: cnt=COUNT(i=1..n, nums[i] mod 2 = 1)";
    green(spec, &["n: 5\nnums: [3, 72, 81, 4, 5]\ncnt: 3"]);
    let msg = red(spec, "n: 5\nnums: [3, 72, 81, 4, 5]\ncnt: 2");
    assert!(
        msg.contains("the test case says 2, the specification gives 3"),
        "{msg}"
    );
}

#[test]
fn baseline3_search_failure_both_conventions() {
    let spec = "In: n∈N, c∈N[1..n,1..4]\nAux: van∈L\nOut: ind∈N\nFn: more5:N→L, more5(i)=FORALL(j=1..4,c[i,j]>5)\nPre: -\nPost: (van,ind)=SEARCH(i=1..n, more5(i))";
    let m = "c: [\n  [5, 6, 7, 4],\n  [6, 4, 6, 8],\n  [1, 2, 3, 4]\n]\n";
    green(
        spec,
        &[
            &format!("n: 3\n{m}van: false\nind: 4"),
            &format!("n: 3\n{m}van: false\nind: 0"),
        ],
    );
}

#[test]
fn task1_libraries() {
    let spec = "In: n∈N, pl∈Tpl[1..n], Tpl=(name:S x x:N x y:N)\nAux: van∈L, ind∈N\nOut: lib∈S\nPre: ∀i∈[1..n]: (pl[i].x≥0 and pl[i].y≥0)\nPost: (van,ind)=SEARCH(i=1..n-1,pl[i].y<pl[i+1].y) and\n      van → lib=pl[ind].name";
    green(
        spec,
        &[
            "n: 5\npl: [\n  {name: \"L1\", x: 12, y: 24},\n  {name: \"L2\", x: 5, y: 15},\n  {name: \"L3\", x: 69, y: 4},\n  {name: \"L4\", x: 12, y: 4},\n  {name: \"L5\", x: 212, y: 47}\n]\nlib: L4\nvan: true\nind: 4",
            "n: 4\npl: [{name: L1, x: 10, y: 50}, {name: L2, x: 20, y: 40}, {name: L3, x: 30, y: 30}, {name: L4, x: 40, y: 20}]\nlib: \"\"\nvan: false\nind: 4",
            "n: 1\npl: [{name: L1, x: 0, y: 10}]\nlib: \"\"\nvan: false\nind: 1",
        ],
    );
}

#[test]
fn task2_matrix_max() {
    let spec = "In: n∈N, m∈N, map∈N[1..n,1..m]\nOut: mpx∈N, mpy∈N\nFn: maxcol: N→N x N, maxcol(i)=MAX(j=1..m,map[i,j])\nPre: ∀i∈[1..n]: (∀j∈[1..m]:(map[i,j]≥0))\nPost: (mpx,)=MAX(i=1..n,maxcol(i).maxval) and\n      mpy=maxcol(mpx).maxind";
    green(
        spec,
        &[
            "n: 6\nm: 4\nmap: [\n  [12, 3, 1, 4],\n  [3, 2, 23, 5],\n  [4, 2, 2, 1],\n  [5, 2, 23, 0],\n  [0, 2, 4, 1],\n  [0, 0, 0, 1]\n]\nmpx: 2\nmpy: 3",
            "n: 2\nm: 2\nmap: [[7, 7], [7, 7]]\nmpx: 1\nmpy: 1",
        ],
    );
}

#[test]
fn task3_mis_with_sum() {
    let spec = "In: n∈N, d∈N, tr∈N[1..n]\nOut: cnt∈N, days∈N[1..cnt]\nFn: total: N→N, total(i)=SUM(j=i-d..i-1,tr[j])\nPre: n>d>0\nPost: (cnt,days)=MIS(i=d+1..n,total(i)<tr[i],i)";
    green(
        spec,
        &[
            "n: 10\nd: 3\ntr: [2, 4, 5, 1, 13, 1, 1, 1, 4, 5]\ncnt: 2\ndays: [5, 9]",
            "n: 5\nd: 2\ntr: [100, 50, 20, 10, 5]\ncnt: 0\ndays: []",
            "n: 4\nd: 3\ntr: [1, 1, 1, 10]\ncnt: 1\ndays: [4]",
        ],
    );
}

#[test]
fn task4_coffee() {
    let spec = "In: n∈N, coffe∈N[1..n,1..4]\nAux: van∈L\nOut: ind∈N\nFn: morethan5:N→L, morethan5(i)=FORALL(j=1..4,coffe[i,j]>5)\nPre: -\nPost: (van,ind)=SEARCH(i=1..n, morethan5(i))";
    green(
        spec,
        &[
            "n: 3\ncoffe: [[4, 3, 5, 6], [4, 7, 5, 6], [5, 6, 5, 6]]\nvan: false\nind: 0",
            "n: 3\ncoffe: [[4, 5, 7, 5], [6, 8, 7, 9], [4, 5, 7, 5]]\nvan: true\nind: 2",
        ],
    );
}

#[test]
fn task5_condmax_slot_skipping() {
    let spec = "In: k∈N, data∈Z[1..]\nOut: van∈L, maxval∈Z\nPre: -\nPost: (van,,maxval)=CONDMAX(i=1..length(data)-1,\n      data[i]-data[i+1],data[i]>k and data[i+1]>k)";
    green(
        spec,
        &[
            "k: 10\ndata: [34, 30, 1, 2, 1, 30, 33, 27, 30, 32]\nvan: true\nmaxval: 6",
            "k: 50\ndata: [34, 30, 1, 2, 1, 30, 33, 27, 30, 32]\nvan: false\nmaxval: 0",
            "k: 10\ndata: [45, 20]\nvan: true\nmaxval: 25",
        ],
    );
}

#[test]
fn bonus1_distinct_count_yaml_records() {
    let spec = "In: n∈N, races∈Race[1..n], Race=(name:S x time:N)\nOut: cnt∈N\nFn: nV:N→L, nV(i)=FORALL(j=1..i-1, races[i].time≠races[j].time)\nPre: -\nPost: cnt=COUNT(i=1..n, nV(i))";
    green(
        spec,
        &[
            "n: 4\nraces:\n  - name: AS\n    time: 37\n  - name: BE\n    time: 31\n  - name: FG\n    time: 30\n  - name: PS\n    time: 37\ncnt: 3",
            "n: 2\nraces: [{name: AS, time: 37}, {name: BE, time: 31}]\ncnt: 2",
        ],
    );
}

#[test]
fn bonus2_mis_projection_with_comment() {
    let spec = "In: Trun=(time:N x loc:S), n∈N, runs∈Trun[1..n]\nOut: cnt∈N, location∈S[1..cnt]\nPre: -\nPost: (cnt,location)=MIS(i=1..n,runs[i].time>30,runs[i].loc)\n/* (cnt,y)=MIS(i=b..e,A(i),f(i))\n   cnt, y ~ cnt, location\n*/";
    green(
        spec,
        &[
            "n: 4\nruns:\n  - time: 15\n    loc: office\n  - time: 35\n    loc: restaurant\n  - time: 40\n    loc: gym\n  - time: 5\n    loc: shop\ncnt: 2\nlocation: [restaurant, gym]",
        ],
    );
}

#[test]
fn bonus3_column_filter() {
    let spec = "In: n∈N, m∈N, k∈N, lates∈N[1..n,1..m]\nOut: cnt∈N, every∈N[1..cnt]\nFn: lt:N→L, lt(j)=FORALL(i=1..n, lates[i,j]>k)\nPre: -\nPost: (cnt, every)=MIS(j=1..m,lt(j))";
    green(
        spec,
        &[
            "n: 3\nm: 4\nk: 5\nlates: [\n  [6, 8, 2, 7],\n  [7, 9, 4, 8],\n  [6, 6, 1, 9]\n]\ncnt: 3\nevery: [1, 2, 4]",
        ],
    );
}

#[test]
fn task6_relational_hungarian() {
    let spec = "Be:\nn∈N,\nar∈N[1..n],\ntav∈N[1..n]\n\nKi:\nmax∈N\n\nEf:\n1<=n and n<=100 and ∀i∈[1..n]:(1<=ar[i] and ar[i]<=2000000 and 1<=tav[i] and tav[i]<=20000)\n\nUf:\n∃i∈[1..n]:(∀j∈[1..n]:(tav[j]<tav[i] vagy (tav[j]=tav[i] és ar[j]>=ar[i])) és ar[i]=max)\n";
    green(
        spec,
        &[
            "n: 6\nar: [30000, 200000, 150000, 38000, 150000, 30000]\ntav: [50, 1900, 2000, 900, 600, 2100]\nmax: 30000",
            "n: 3\nar: [500, 250, 100]\ntav: [1000, 1000, 500]\nmax: 250",
            "n: 1\nar: [1]\ntav: [1]\nmax: 1",
            "n: 4\nar: [100, 100, 2000000, 1]\ntav: [20000, 20000, 20000, 20000]\nmax: 1",
            // output omitted: the relational formula computes it
            "n: 3\nar: [500, 250, 100]\ntav: [1000, 1000, 500]",
        ],
    );
    let msg = red(
        spec,
        "n: 3\nar: [500, 250, 100]\ntav: [1000, 1000, 500]\nmax: 500",
    );
    assert!(
        msg.contains("the test case says 500, the specification gives 250"),
        "{msg}"
    );
}

#[test]
fn task7_8_darab_and_filtered_sum() {
    green(
        "Be: szam∈N\n\nKi: osztok∈N\n\nEf: szam>=1\n\nUf: osztok=DARAB(i=1..szam, i|szam)    // szam mod i = 0",
        &[
            "szam: 12\nosztok: 6",
            "szam: 1\nosztok: 1",
            "szam: 13\nosztok: 2",
        ],
    );
    green(
        "Be: n∈N\n\nKi: sum∈N\n\nEf: n>=1\n\nUf: sum = SUM(i=1..n, i, i%2=1 and i|n)",
        &["n: 10\nsum: 6", "n: 1\nsum: 1"],
    );
}

#[test]
fn task9_lookup_table() {
    let spec = "Be:\n\nlapok∈S[1..8]=[\"also\",\"felso\",\"kiraly\",\"VII\", \"VIII\", \"IX\", \"X\", \"asz\"],\n\nertekek∈N[1..8]=[2,3,4,7,8,9,10,11], lap∈S\n\nKi:\n\nertek∈N\n\nEf:\n\nlap!=\"\" and ∃i∈[1..8]:(lapok[i]=lap)\n\nUf:\n\n∃i∈[1..8]:(lapok[i]=lap and ertek=ertekek[i])";
    green(
        spec,
        &[
            "lapok: [\"also\", \"felso\", \"kiraly\", \"VII\", \"VIII\", \"IX\", \"X\", \"asz\"]\nertekek: [2, 3, 4, 7, 8, 9, 10, 11]\nlap: \"asz\"\nertek: 11",
            "lap: also\nertek: 2",
        ],
    );
}

#[test]
fn task10_relational_and_macro() {
    let rel = "Be:\n\nn∈N, k∈N, ar∈N[1..n]\n\nKi:\n\nsorszam∈Z\n\nEf:\n\n1<=n<=100 and 1<=k<=10000 and ∀i∈[1..n]:(1<=ar[i] and ar[i]<=10000)\n\nUf:\n\n(sorszam=-1 and ∀i∈[1..n]:(ar[i]>=k)) or ∃i∈[1..n]:(sorszam=i and ar[i]<k and ∀j∈[1..i-1]:(ar[j]>=k))";
    green(
        rel,
        &[
            "n: 6\nk: 2000\nar: [2500, 5300, 1900, 2400, 8800, 5300]\nsorszam: 3",
            "n: 3\nk: 100\nar: [2500, 5300, 1900]\nsorszam: -1",
            "n: 4\nk: 5000\nar: [6000, 3000, 1000, 200]\nsorszam: 2",
            "n: 4\nk: 1\nar: [1, 1, 10000, 7]\nsorszam: -1",
            "n: 1\nk: 3000\nar: [2500]\nsorszam: 1",
        ],
    );
    let mac = "Be:\nn∈N,\nk∈N,\nar∈N[1..n],\negyeb∈N\n\nSa:\nvan∈L,\nind∈N\n\nKi:\nsorszam∈Z,\ndb∈N\n\nFv:\nkisebb:N->L,\nkisebb(i)=ar[i]<k\n\nEf:\n1<=n<=100 and 1<=k<=10000 and egyeb>=0 and ∀i∈[1..n]:(1<=ar[i] and ar[i]<=10000)\n\nUf:\n(van,ind)=SEARCH(i=1..n, kisebb(i)) and\n(van -> sorszam=ind) and\n(not van -> sorszam=-1) and\ndb=DARAB(i=1..n, ar[i]<k) and\negyeb=egyeb";
    green(
        mac,
        &[
            "n: 6\nk: 2000\nar: [2500, 5300, 1900, 2400, 8800, 1000]\negyeb: 0\nvan: true\nind: 3\nsorszam: 3\ndb: 2",
            "n: 3\nk: 100\nar: [2500, 5300, 1900]\negyeb: 5\nvan: false\nind: 4\nsorszam: -1\ndb: 0",
            "n: 4\nk: 10000\nar: [1, 2, 3, 4]\negyeb: 0\nsorszam: 1\ndb: 4",
        ],
    );
}

#[test]
fn misc_language_features() {
    // reverse search, COPY with ABS, tuples, membership, chained comparisons, implication
    green(
        "In: n∈N, nums∈Z[1..n], a∈N\nAux: van∈L\nOut: ind∈N, abs∈Z[1..n]\nPre: a∈[2..7] and 2≤a≤7 and not(true → false) and (3, 5).1=3\nPost: (van,-ind)=SEARCH(i=-n..-1, nums[-i] mod 2 = 1) and abs=COPY(i=1..n, ABS(nums[i])) and a=a",
        &["n: 4\nnums: [1, -3, 4, 6]\na: 3\nvan: true\nind: 2\nabs: [1, 3, 4, 6]"],
    );
    // zero-based open array + from/to + MIN
    green(
        "In: x∈Z[0..]\nOut: m∈Z\nPre: length(x)>0\nPost: (,m)=MIN(i=from(x)..to(x), x[i])",
        &["x: [5, -2, 7]\nm: -2"],
    );
}

// ---------------------------------------------------------------- error messages

#[test]
fn err_reserved_exists() {
    let m = spec_err("In: n∈N\nAux: exists∈L\nOut: ind∈N\nPre: -\nPost: n=n");
    assert!(
        m.contains("`exists` is a reserved word") && m.contains("van"),
        "{m}"
    );
    assert!(m.contains("line 2"), "{m}");
}

#[test]
fn err_mandate() {
    let m = spec_err("Be: n∈N, egyeb∈N\nKi: x∈N\nEf: egyeb>0\nUf: x=n");
    assert!(
        m.contains("Input `egyeb` is never used")
            && m.contains("and egyeb=egyeb")
            && m.contains("line 1"),
        "{m}"
    );
}

#[test]
fn err_wrong_header() {
    let m = spec_err("Be: n∈N\nSegéd: van∈L\nKi: x∈N\nEf: -\nUf: x=n");
    assert!(
        m.contains("Unknown block header `Segéd:`") && m.contains("`Sa:`"),
        "{m}"
    );
    let m = spec_err("in: n∈N\nOut: x∈N\nPre: -\nPost: x=n");
    assert!(m.contains("case-sensitive") && m.contains("`In:`"), "{m}");
    let m = spec_err("In:n∈N\nOut: x∈N\nPre: -\nPost: x=n");
    assert!(m.contains("Missing space after `In:`"), "{m}");
}

#[test]
fn err_block_order_and_missing() {
    let m = spec_err("Be: n∈N\nEf: -\nKi: x∈N\nUf: x=n");
    assert!(m.contains("`Ki:` is in the wrong place"), "{m}");
    let m = spec_err("Be: n∈N\nKi: x∈N\nUf: x=n");
    assert!(
        m.contains("Missing `Ef:` block") && m.contains("Ef: -"),
        "{m}"
    );
}

#[test]
fn err_syntax_messages() {
    let m = spec_err("In: n∈N\nOut: x∈N\nPre: -\nPost: x=(n+1");
    assert!(m.contains("to close the `(` opened at line 4"), "{m}");
    let m = spec_err("In: n∈N\nOut: x∈N\nPre: n>0 n<5\nPost: x=n");
    assert!(m.contains("join them with `and`"), "{m}");
    let m = spec_err("In: n∈N\nOut: x∈N\nPre: -\nPost: x=sum(i=1..n, i)");
    assert!(
        m.contains("Unknown function `sum`") && m.contains("SUM"),
        "{m}"
    );
    let m = spec_err("In: n∈N\nOut: x∈N\nPre: -\nPost: x=nn");
    assert!(
        m.contains("`nn` is not declared") && m.contains("Did you mean `n`"),
        "{m}"
    );
    let m = spec_err("In: n∈N\nOut: x∈N\nPre: -\nPost: x:=n");
    assert!(m.contains("`=` instead of `:=`"), "{m}");
}

#[test]
fn err_test_data() {
    let spec = "Be: n∈N, ar∈N[1..n]\nKi: sorszam∈N\nEf: -\nUf: sorszam=n and ar=ar";
    let m = red(spec, "n: 1\nar: [1]\nsorszam: -1");
    assert!(m.contains("declare it as Z"), "{m}");
    let m = red(spec, "n: 3\nar: [1, 2]\nsorszam: 3");
    assert!(m.contains("has 2 element(s)") && m.contains("n = 3"), "{m}");
    let m = red(spec, "ar: [1]\nsorszam: 3");
    assert!(m.contains("Input `n` is missing"), "{m}");
    let m = red(spec, "n: 1\nAr: [1]\nar: [1]\nsorszam: 1");
    assert!(
        m.contains("`Ar` is not declared") && m.contains("Did you mean `ar`"),
        "{m}"
    );
}

#[test]
fn err_pre_and_runtime() {
    let spec = "Be: n∈N, k∈N, ar∈N[1..n]\nKi: x∈N\nEf: 1<=k<=10000 and ∀i∈[1..n]:(ar[i]>=1)\nUf: x=ar[n+1] and k=k";
    let m = red(spec, "n: 1\nk: 0\nar: [1]\nx: 1");
    assert!(
        m.contains("`1<=k<=10000` does not hold") && m.contains("k = 0"),
        "{m}"
    );
    let m = red(spec, "n: 2\nk: 1\nar: [1, 0]\nx: 1");
    assert!(m.contains("fails for i = 2"), "{m}");
    let m = red(spec, "n: 2\nk: 1\nar: [1, 2]\nx: 1");
    assert!(
        m.contains("index 3 is out of range") && m.contains("1..2"),
        "{m}"
    );
}
