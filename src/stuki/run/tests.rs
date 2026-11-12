use super::super::from_spec::from_spec;
use super::super::model::{Case, fresh_id};
use super::super::rich::Rich;
use super::*;
use crate::parser::parse;

const COUNT: &str = "Be:\nn∈N,\nx∈Z[1..n]\nKi:\ndb∈N\nEf:\n-\nUf:\ndb=DARAB(i=1..n, x[i]>0)\n";
const DATA: &str = "n: 4\nx: [3, -1, 0, 8]\ndb: 2";

fn node(kind: Kind) -> Node {
    Node {
        id: fresh_id(),
        bg: None,
        kind,
    }
}

fn st(text: &str) -> Node {
    node(Kind::Statement(Rich::plain(text)))
}

fn when(cond: &str, yes: Vec<Node>, no: Vec<Node>) -> Node {
    node(Kind::If {
        cond: Rich::plain(cond),
        yes,
        no,
    })
}

fn during(cond: &str, body: Vec<Node>) -> Node {
    node(Kind::While {
        cond: Rich::plain(cond),
        body,
    })
}

fn program(diagrams: Vec<(&str, &str, Vec<Node>)>) -> Program {
    Program {
        diagrams: diagrams
            .into_iter()
            .map(|(name, ret, body)| Diagram {
                name: Rich::plain(name),
                ret: Rich::plain(ret),
                show_header: true,
                body,
                ..Diagram::new()
            })
            .collect(),
        ..Program::default()
    }
}

fn run(p: &Program, spec: &str, data: &str) -> Verdict {
    let spec = parse(spec).unwrap_or_else(|e| panic!("{}", e[0].title));
    run_tests(p, &spec, &[data.to_string()], &AtomicBool::new(false)).remove(0)
}

fn counting(step: &str) -> Program {
    program(vec![(
        "Számlálás",
        "",
        vec![
            st("db:=0"),
            st("i:=1"),
            during(
                "i≤n",
                vec![
                    when("x[i]>0", vec![st("db:=db+1")], vec![st("-")]),
                    st(step),
                ],
            ),
        ],
    )])
}

#[test]
fn a_hand_written_structogram_passes_or_fails() {
    let v = run(&counting("i:=i+1"), COUNT, DATA);
    assert!(v.pass, "{}", v.message);
    assert_eq!(v.values, [("db".to_string(), "2".to_string())]);

    let wrong = program(vec![("P", "", vec![st("db:=n")])]);
    let v = run(&wrong, COUNT, DATA);
    assert!(!v.pass);
    assert!(
        v.message.contains("postcondition (Uf) is false"),
        "{}",
        v.message
    );
    assert_eq!(v.values[0].1, "4");

    let v = run(&program(vec![("P", "", vec![st("-")])]), COUNT, DATA);
    assert_eq!(
        v.message,
        "`db` never gets a value (the specification gives 2)"
    );
}

#[test]
fn the_element_that_stops_the_run_is_named() {
    let p = counting("i:=i");
    let v = run(&p, COUNT, DATA);
    assert!(
        v.message.starts_with("The structogram doesn't stop"),
        "{}",
        v.message
    );

    let p = program(vec![(
        "P",
        "",
        vec![st("db:=0"), st("db:=x[n+1]"), st("db:=1")],
    )]);
    let v = run(&p, COUNT, DATA);
    assert_eq!(v.node, Some(p.diagrams[0].body[1].id));
    assert!(
        v.message.starts_with("`x[n+1]`: ") && v.message.contains("out of range"),
        "{}",
        v.message
    );

    let v = run(&program(vec![("P", "", vec![st("db:=k+1")])]), COUNT, DATA);
    assert_eq!(v.message, "`k+1`: `k` has no value at this point");

    let v = run(
        &program(vec![("P", "", vec![when("db=0", vec![], vec![])])]),
        COUNT,
        DATA,
    );
    assert_eq!(v.message, "`db=0`: `db` has no value at this point");
}

#[test]
fn texts_that_are_not_steps_are_reported_before_running() {
    let p = program(vec![(
        "P",
        "",
        vec![st("db:=0"), st("count the positives")],
    )]);
    let v = run(&p, COUNT, DATA);
    assert_eq!(v.node, Some(p.diagrams[0].body[1].id));
    assert!(v.message.contains("can't be run"), "{}", v.message);

    let v = run(&program(vec![("P", "", vec![st("db<n")])]), COUNT, DATA);
    assert!(v.message.contains("is not a step"), "{}", v.message);
    let v = run(&program(vec![("P", "", vec![st("db:=f(n)")])]), COUNT, DATA);
    assert!(v.message.contains("`f` is not a function"), "{}", v.message);
    let v = run(&program(vec![("P", "", vec![st("3:=n")])]), COUNT, DATA);
    assert!(v.message.starts_with("Only a variable"), "{}", v.message);
    let v = run(
        &program(vec![("P", "", vec![during("i:=1", vec![])])]),
        COUNT,
        DATA,
    );
    assert!(v.message.contains("is not a condition"), "{}", v.message);
    let v = run(
        &Program {
            diagrams: vec![],
            ..Program::default()
        },
        COUNT,
        DATA,
    );
    assert_eq!(v.message, "There is no structogram to run");
}

#[test]
fn test_cases_the_specification_rejects_are_not_run() {
    let spec = COUNT.replace("Ef:\n-", "Ef:\nn>9");
    let v = run(&counting("i:=i+1"), &spec, DATA);
    assert_eq!(
        v.message,
        "The test case's input doesn't satisfy the precondition"
    );
    let v = run(&counting("i:=i+1"), COUNT, "n: [");
    assert!(!v.pass && !v.message.is_empty());
}

#[test]
fn functions_are_called_by_their_header() {
    let spec = "Be:\nn∈N,\nx∈Z[1..n]\nKi:\ns∈Z,\nh∈N\nFv:\nketszer:Z->Z, ketszer(a)=2*a\nEf:\nn>0\nUf:\ns=SZUMMA(i=1..n, ketszer(x[i])) és (h,)=MAX(i=1..n, x[i])\n";
    let main = vec![
        st("s:=0"),
        during("i=1..n", vec![st("s:=s+ketszer(x[i])")]),
        st("(h, e):=legnagyobb(n)"),
    ];
    let largest = vec![
        st("mi:=1"),
        during(
            "i=2..k",
            vec![when("x[i]>x[mi]", vec![st("mi:=i")], vec![])],
        ),
    ];
    let p = program(vec![
        ("ketszer(a:Egész):Egész", "a+a", vec![st("-")]),
        ("Főprogram", "", main),
        ("legnagyobb(k:Egész):Egész x Egész", "(mi, x[mi])", largest),
    ]);
    let v = run(&p, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
    assert!(v.pass, "{}", v.message);
    assert_eq!(
        v.values,
        [("s".into(), "22".into()), ("h".into(), "2".to_string())]
    );

    let mut wrong = p.clone();
    wrong.diagrams[0].ret = Rich::plain("a");
    let v = run(&wrong, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
    assert!(!v.pass && v.values[0].1 == "11", "{v:?}");

    let mut endless = p.clone();
    endless.diagrams[0].ret = Rich::plain("ketszer(a)");
    let v = run(&endless, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
    assert!(
        v.message.contains("calls itself too deeply"),
        "{}",
        v.message
    );

    let mut silent = p.clone();
    silent.diagrams[0].show_return = false;
    let v = run(&silent, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
    assert!(v.message.contains("has no return value"), "{}", v.message);
}

#[test]
fn arrays_records_loops_and_cases() {
    let spec = "In:\nn∈N\nOut:\ny∈N[0..n],\nm∈N[1..2,1..2],\np∈P,\nP=(a:N x b:N),\nk∈N\nPre:\n-\nPost:\n∀i∈[0..n]:(y[i]=i*i) and m=[[1,2],[3,4]] and p.a=n and p.b=1 and k=3\n";
    let fill = vec![st("m[r,c]:=(r-1)*2+c")];
    let cases = node(Kind::Switch {
        cases: vec![
            Case {
                id: fresh_id(),
                cond: Some(Rich::plain("n<0")),
                body: vec![st("k:=0")],
            },
            Case {
                id: fresh_id(),
                cond: None,
                body: vec![st("k:=0")],
            },
        ],
    });
    let body = vec![
        during("i=0..n", vec![st("y[i]:=i*i")]),
        during("r=1..2", vec![during("c=1..2", fill)]),
        st("p.a:=n"),
        st("p.b:=1"),
        cases,
        node(Kind::Repeat {
            cond: Rich::plain("k<3"),
            body: vec![st("k:=k+1")],
        }),
    ];
    let p = program(vec![("P", "", body)]);
    let v = run(
        &p,
        spec,
        "n: 2\ny: [0, 1, 4]\nm: [[1, 2], [3, 4]]\np: {a: 2, b: 1}\nk: 3",
    );
    assert!(v.pass, "{}", v.message);
    assert_eq!(v.values[0], ("y".into(), "[0, 1, 4]".into()));

    let none = node(Kind::Switch {
        cases: vec![Case {
            id: fresh_id(),
            cond: Some(Rich::plain("n<0")),
            body: vec![],
        }],
    });
    let p = program(vec![("P", "", vec![none])]);
    let v = run(
        &p,
        spec,
        "n: 2\ny: [0, 1, 4]\nm: [[1, 2], [3, 4]]\np: {a: 2, b: 1}\nk: 3",
    );
    assert_eq!(v.node, Some(p.diagrams[0].body[0].id));
    assert_eq!(
        v.message,
        "None of the cases of the multi-way decision holds"
    );
}

#[test]
fn generated_structograms_pass_their_test_cases() {
    let table = "Be:\nn∈Z,\nm∈Z,\nx∈Z[1..n],\nt∈Z[1..n,1..m]\nKi:\n";
    let rest = " és n=n és m=m és x=x és t=t\n";
    let data = "n: 3\nm: 2\nx: [5, -2, 7]\nt: [[0, 0], [1, -4], [2, 3]]\n";
    let cases = [
        (
            "van∈L\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(SZUMMA(j=1..m, t[i,j])>0)",
            "van: true",
        ),
        (
            "van∈L\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(SZUMMA(j=1..m, t[i,j])>9)",
            "van: false",
        ),
        (
            "van∈L\nEf:\n-\nUf:\nvan=∀i∈[1..n]:(DARAB(j=1..m, t[i,j]>0)>0)",
            "van: false",
        ),
        (
            "van∈L\nEf:\n-\nUf:\nvan=∀i∈[1..n]:(DARAB(j=1..m, t[i,j]>9)=0)",
            "van: true",
        ),
        (
            "van∈L,\nk∈N\nEf:\n-\nUf:\n(van,k)=KERES(i=1..n, SZUMMA(j=1..m, t[i,j])>3)",
            "van: true\nk: 3",
        ),
        (
            "van∈L,\nk∈N\nEf:\n-\nUf:\n(van,k)=KERES(i=1..n, SZUMMA(j=1..m, t[i,j])>30)",
            "van: false\nk: 0",
        ),
        (
            "k∈N\nEf:\n-\nUf:\nk=KIVÁLASZT(i>=1, SZUMMA(j=1..m, t[i,j])<0)",
            "k: 2",
        ),
        (
            "mi∈N,\nme∈Z\nEf:\nn>0\nUf:\n(mi,me)=MAX(i=1..n, SZUMMA(j=1..m, t[i,j]))",
            "mi: 3\nme: 5",
        ),
        (
            "mi∈N,\nme∈Z\nEf:\nn>0\nUf:\n(mi,me)=MIN(i=1..n, SZUMMA(j=1..m, t[i,j]))",
            "mi: 2\nme: -3",
        ),
        (
            "s∈Z\nEf:\n-\nUf:\ns=SZUMMA(i=1..n, SZUMMA(i=1..m, t[2,i]))",
            "s: -9",
        ),
        ("s∈Z\nEf:\n-\nUf:\n(n>m -> s=n) és (n≤m -> s=m)", "s: 3"),
        ("s∈Z\nEf:\n-\nUf:\n(n<m -> s=n) és (m≤n -> s=m)", "s: 2"),
        (
            "s∈Z\nEf:\n-\nUf:\n(n>3 -> s=1) és (n=3 -> s=0) és (n<3 -> s=-1)",
            "s: 0",
        ),
        (
            "van∈L,\nmi∈N,\nme∈Z\nEf:\n-\nUf:\n(van,mi,me)=MAXHA(i=1..n, x[i], DARAB(j=1..m, t[i,j]>0)>0)",
            "van: true\nmi: 3\nme: 7",
        ),
        (
            "s∈Z\nEf:\n-\nUf:\n(DARAB(i=1..n, x[i]>0)>1 -> s=1) és (DARAB(i=1..n, x[i]>0)≤1 -> s=0)",
            "s: 1",
        ),
        (
            "db∈N,\ny∈Z[1..db]\nEf:\n-\nUf:\n(db,y)=KIVÁLOGAT(i=1..n, x[i]>0, x[i]*2)",
            "db: 2\ny: [10, 14]",
        ),
        (
            "y∈Z[1..n]\nEf:\n-\nUf:\ny=MÁSOL(i=1..n, SZUMMA(j=1..m, t[i,j]))",
            "y: [0, -3, 5]",
        ),
        // shapes that used to be left as red statements
        (
            "y∈Z[1..n]\nEf:\n-\nUf:\ny[1]=x[1] és ∀i∈[2..n]:(y[i]=y[i-1]+x[i])",
            "y: [5, 3, 10]",
        ),
        (
            "y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:((x[i]>0 -> y[i]=1) és (x[i]≤0 -> y[i]=0))",
            "y: [1, 0, 1]",
        ),
        (
            "y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:(x[i]>0 -> y[i]=x[i]) és ∀i∈[1..n]:(x[i]≤0 -> y[i]=0)",
            "y: [5, 0, 7]",
        ),
        (
            "y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..2]:(y[i]=x[i]) és ∀i∈[3..n]:(y[i]=0)",
            "y: [5, -2, 0]",
        ),
        ("s∈Z\nEf:\n-\nUf:\ns=SZUMMA(i=1..n, x[i]) és s≥0", "s: 10"),
    ];
    for (tail, outputs) in cases {
        let text = format!("{table}{tail}{rest}");
        let spec = parse(&text).unwrap_or_else(|e| panic!("{tail}: {}", e[0].title));
        let test = format!("{data}{outputs}");
        assert!(
            crate::run_test(&spec, &test).pass,
            "{tail}: the test case itself fails"
        );
        let built = from_spec(&spec, "");
        assert!(built.unset.is_empty(), "{tail}");
        let v = run_tests(&built.program, &spec, &[test], &AtomicBool::new(false)).remove(0);
        assert!(v.pass, "{tail}\n{}\n{:?}", v.message, v.values);
    }
}
