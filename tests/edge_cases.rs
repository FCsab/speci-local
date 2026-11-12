//! Edge cases found in review: inputs that used to panic, mislead or cost far too much.

use speci_local::{parse, run_test, scaffold};

fn render(d: &[speci_local::diag::Diagnostic]) -> String {
    d.iter()
        .map(|d| d.render("", ""))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn spec(src: &str) -> speci_local::parser::Spec {
    parse(src).unwrap_or_else(|d| panic!("spec error:\n{}", render(&d)))
}

fn red(src: &str, test: &str) -> String {
    let o = run_test(&spec(src), test);
    assert!(!o.pass, "expected red for\n{test}");
    render(&o.diags)
}

#[test]
fn comment_markers_inside_any_text_literal() {
    for lit in [
        "\"a//b\"",
        "'a//b'",
        "„a//b”",
        "\"a/*b\"",
        "'/*'",
        "\"it's // fine\"",
    ] {
        let src = format!("Be: s∈S\nKi: b∈L\nEf: -\nUf: b=(s={lit}) // a real comment");
        let o = run_test(&spec(&src), "s: x\nb: false");
        assert!(o.pass, "{lit}: {}", render(&o.diags));
    }
    let m = render(&parse("Be: s∈S\nKi: b∈L\nEf: -\nUf: b=(s='a\n// c\n)").unwrap_err());
    assert!(m.contains("Unterminated text literal"), "{m}");
}

#[test]
fn text_closed_by_a_typographic_quote() {
    let s = spec("Be: s∈S\nKi: b∈L\nEf: -\nUf: b=(s=\"abc”) and b=b");
    assert!(run_test(&s, "s: abc\nb: true").pass);
    assert!(!run_test(&s, "s: abd\nb: true").pass);
}

#[test]
fn hungarian_logical_values_in_test_data() {
    let s = spec("Be: s∈S\nKi: b∈L\nEf: -\nUf: b=(s=\"abc\")");
    assert!(run_test(&s, "s: abc\nb: igaz").pass);
    assert!(run_test(&s, "s: abd\nb: hamis").pass);
    assert!(!run_test(&s, "s: abd\nb: igaz").pass);
}

#[test]
fn extreme_integers_are_errors_not_panics() {
    const MIN: &str = "-9223372036854775808";
    let m = red(
        "Be: a∈Z\nKi: x∈Z\nEf: -\nUf: x=abs(a)",
        &format!("a: {MIN}\nx: 0"),
    );
    assert!(m.contains("Number overflow"), "{m}");
    let _ = red(
        "Be: a∈Z\nKi: x∈Z\nEf: -\nUf: -x=a",
        &format!("a: {MIN}\nx: 0"),
    );
    let big = "9223372036854775807";
    let src = format!("Be: n∈N\nKi: x∈Z\nEf: -\nUf: x=to([{big}..:1, 2]) and n=n");
    let _ = run_test(&spec(&src), "n: 1\nx: 0");
    let src = format!("Be: n∈N\nKi: x∈Z\nEf: -\nUf: x=[{big}..:1, 2][n]");
    let m = red(&src, "n: 1\nx: 0");
    assert!(m.contains("out of range"), "{m}");
    let src = format!("Be: n∈N\nKi: b∈L\nEf: -\nUf: b=([{big}..:1, 2]=[1, 2, 3]) and n=n");
    let m = red(&src, "n: 1\nb: true");
    assert!(m.contains("same index range"), "{m}");
    let src = format!("Be: x∈N[-{big}..{big}]\nKi: b∈L\nEf: -\nUf: b=(x=x)");
    let m = red(&src, "x: [1]\nb: true");
    assert!(m.contains("needs exactly"), "{m}");
}

#[test]
fn false_quantifiers_are_explained() {
    let src =
        "Be: n∈N, ar∈N[1..n]\nKi: ok∈L\nEf: ∀i∈[1..n]:(ar[i]>0)\nUf: ok=true and n=n and ar=ar";
    let m = red(src, "n: 4\nar: [5, 6, 0, 0]\nok: true");
    assert!(
        m.contains("It fails for i = 3: ar = [5, 6, 0, 0], i = 3"),
        "{m}"
    );
    let src =
        "Be: n∈N, ar∈N[1..n]\nKi: ok∈L\nEf: ∃i∈[1..n]:(ar[i]>9)\nUf: ok=true and n=n and ar=ar";
    let m = red(src, "n: 2\nar: [5, 6]\nok: true");
    assert!(m.contains("No i in 1..2 makes it true."), "{m}");
    let m = red(src, "n: 0\nar: []\nok: true");
    assert!(m.contains("The range 1..0 is empty, so ∃ is false."), "{m}");
    let src = "Be: n∈N\nKi: ok∈L\nEf: n>5\nUf: ok=true and n=n";
    let m = red(src, "n: 2\nok: true");
    assert!(m.contains("Values used: n = 2."), "{m}");
}

#[test]
fn failed_tests_show_what_the_specification_gives() {
    let s = spec("Be: n∈N\nKi: d∈N, t∈N\nEf: -\nUf: d=n*2 and t=n*3");
    let o = run_test(&s, "n: 4\nd: 8\nt: 12");
    assert!(o.pass);
    assert_eq!(
        o.values,
        [
            ("d".into(), "8".into(), true),
            ("t".into(), "12".into(), true)
        ]
    );
    let o = run_test(&s, "n: 4\nd: 9\nt: 12");
    assert!(!o.pass);
    assert_eq!(
        o.values,
        [
            ("d".into(), "8".into(), false),
            ("t".into(), "12".into(), false)
        ]
    );
    let m = render(&o.diags);
    assert!(
        m.contains("`d`: the test case says 9, the specification gives 8"),
        "{m}"
    );
    assert!(!m.contains("`t`:"), "{m}");
    let s = spec("Be: n∈N\nKi: d∈N\nEf: -\nUf: d=n and d=n+1");
    let o = run_test(&s, "n: 1\nd: 1");
    assert_eq!(o.values, [("d".into(), "1".into(), true)]);
    assert!(render(&o.diags).contains("Even with the expected outputs removed"));
}

#[test]
fn every_declared_variable_must_be_in_the_test() {
    let s = spec("Be: n∈N\nSa: h∈N\nKi: d∈N\nEf: -\nUf: h=n and d=h");
    for (data, missing) in [
        ("h: 1\nd: 1", "Input `n` is missing"),
        ("n: 1\nd: 1", "Auxiliary variable `h` is missing"),
        ("n: 1\nh: 1", "Output `d` is missing"),
    ] {
        let o = run_test(&s, data);
        assert!(!o.pass && o.values.is_empty());
        assert!(render(&o.diags).contains(missing), "{data}");
    }
}

#[test]
fn array_builtins_look_at_variables_in_place() {
    let s =
        spec("Be: x∈Z[0..]\nKi: a∈Z, b∈Z, c∈Z\nEf: -\nUf: a=length(x) and b=from(x) and c=to(x)");
    assert!(run_test(&s, "x: [7, 8, 9]\na: 3\nb: 0\nc: 2").pass);
    assert!(run_test(&s, "x: []\na: 0\nb: 0\nc: -1").pass);
    let m = red("Be: n∈N\nKi: a∈Z\nEf: -\nUf: a=length(n)", "n: 3\na: 1");
    assert!(
        m.contains("`length` needs an array, but `n` is a whole number"),
        "{m}"
    );
    let s = spec("Be: s∈S\nKi: a∈Z\nEf: -\nUf: a=length(s)+length([1, 2])");
    assert!(run_test(&s, "s: héj\na: 5").pass);
    let s = spec("Be: n∈N, x∈N[1..n]\nKi: ok∈L\nEf: -\nUf: ok=∀i∈[1..n]:(i<=length(x)) and n=n");
    let xs = vec!["1"; 20_000].join(", ");
    let t = std::time::Instant::now();
    assert!(run_test(&s, &format!("n: 20000\nx: [{xs}]\nok: true")).pass);
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
}

#[test]
fn loading_matrices_does_not_copy_earlier_ones() {
    let rows = 400;
    let row = format!("[{}]", vec!["1"; rows].join(", "));
    let matrix = format!("[{}]", vec![row.as_str(); rows].join(", "));
    let s = spec("Be: n∈N, a∈N[1..n,1..n], b∈N[1..n,1..n]\nKi: ok∈L\nEf: -\nUf: ok=(a=b) and n=n");
    let data = format!("n: {rows}\na: {matrix}\nb: {matrix}\nok: true");
    let t = std::time::Instant::now();
    assert!(run_test(&s, &data).pass);
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
}

#[test]
fn scaffolds_missing_variables() {
    let s = spec(
        "Be: n∈N, m∈N[1..n,1..n], k∈N=3\nSa: r∈Rec, Rec=(name:S x age:N)\nKi: xs∈N[], d∈N\nEf: -\nUf: d=n+k and m=m and r=r and xs=xs",
    );
    assert_eq!(
        scaffold(&s, ""),
        "n: \nm: [[]]\nr: {name: , age: }\nxs: []\nd: \n"
    );
    assert_eq!(
        scaffold(&s, "n: 2\n  m : [[1]]\nnot_d: 1"),
        "r: {name: , age: }\nxs: []\nd: \n"
    );
    assert_eq!(scaffold(&s, "n: 1\nm: []\nr: {}\nxs: []\nd: 4"), "");
}

#[test]
fn diagnostics_render_with_a_caret() {
    let src = "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=nn+n";
    let d = parse(src).unwrap_err();
    assert_eq!(
        d[0].render(src, ""),
        "Specification, line 4, column 7: `nn` is not declared\n 4 | Uf: x=nn+n\n   |       ^\nEvery variable must be declared in an In/Aux/Out (Be/Sa/Ki) block. Did you mean `n`?"
    );
    let s = spec("Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n");
    let data = "n: 1\nx: -1";
    let o = run_test(&s, data);
    let text = o.diags[0].render(src, data);
    assert!(text.starts_with("Test data, line 2, column 1:"), "{text}");
    assert!(text.contains(" 2 | x: -1\n   | ^"), "{text}");
    let o = run_test(&s, "n: 1");
    let text = o.diags[0].render(src, "n: 1");
    assert!(
        text.starts_with("Output `x` is missing from this test case\n"),
        "{text}"
    );
    assert!(text.contains("\nFix: Add a line `x: …`"), "{text}");
}

#[test]
fn the_stepper_stops_evaluating_once_it_has_its_steps() {
    use speci_local::eval::TRACE_CAP;
    use speci_local::trace_test;
    use std::sync::atomic::AtomicBool;
    let spec =
        parse("In: n∈N\nOut: c∈N\nPre: -\nPost: c=COUNT(i=1..n, COUNT(j=1..n, i<j)>0)").unwrap();
    let started = std::time::Instant::now();
    let trace = trace_test(&spec, "n: 3000\nc: 2999", &AtomicBool::new(false)).unwrap();
    assert_eq!(trace.steps.len(), TRACE_CAP);
    assert!(trace.truncated);
    assert!(
        started.elapsed().as_secs() < 5,
        "the evaluation went on after the last recorded step"
    );
    let trace = trace_test(&spec, "n: 3\nc: 2", &AtomicBool::new(false)).unwrap();
    assert!(!trace.truncated && trace.steps.len() < TRACE_CAP);
}

#[test]
fn arrays_and_records_are_shared_but_compare_by_value() {
    let spec = parse(
        "In: n∈N, x∈Z[1..n], m∈Z[1..2,1..n], p∈T[1..n], T=(a:N x b:S)\nOut: ok∈L\nFn: same:Z[1..n] x Z[1..n]->L, same(u,v)=u=v\nPre: -\nPost: ok=(same(x,x) and m[1]=x and m[2]≠x and m[1][2]=m[1,2] and p[1]=p[2] and p[1]≠p[3] and p[3].b=\"z\") and n=n",
    )
    .unwrap();
    let data = "n: 3\nx: [1, 2, 3]\nm: [[1, 2, 3], [1, 2, 4]]\np: [{a: 1, b: y}, {b: y, a: 1}, {a: 1, b: z}]\nok: true";
    let o = run_test(&spec, data);
    assert!(o.pass, "{:?}", o.diags);
}

/// Data typed over several rows, each starting at the left edge: YAML alone rejects that.
#[test]
fn data_rows_may_start_at_the_left_edge() {
    let src = "Be: n∈N, x∈Z[1..n], d∈D[1..n], D=(kor:N x fizetes:N), s∈S\nKi: m∈Z\nEf: -\nUf: m=SZUMMA(i=1..n, x[i]+d[i].kor) és s=s\n";
    for data in [
        "n: 2\nx: [1,\n2]\nd: [\n{kor: 3, fizetes: 0},\n{kor: 4,\nfizetes: 0}\n]\ns: x\nm: 10",
        // brackets in a quoted text or a comment open nothing; `O'Brien` is no quote
        "n: 2\ns: \"a[\"\nx: [1, 2] # [\nd: [{kor: 3, fizetes: 0}, {kor: 4, fizetes: 0}]\nm: 10",
        "n: 2\ns: O'Brien[\nx: [1, 2]\nd: [{kor: 3, fizetes: 0}, {kor: 4, fizetes: 0}]\nm: 10",
    ] {
        let o = run_test(&spec(src), data);
        assert!(o.pass, "{data}\n{}", render(&o.diags));
    }
    // an error in such a row points where it is in the text as written
    let m = red(src, "n: 2\nx: [1,\n2}\nd: []\ns: x\nm: 0");
    assert!(m.contains("line 3, column 2"), "{m}");
}
