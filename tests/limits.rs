//! Formulas are parsed and evaluated recursively, so their depth is limited: whatever the text,
//! the answer is a message, never a stack overflow (which would take the whole editor down).

use speci_local::eval::EVAL_STACK;
use speci_local::{parse, run_test};

fn verdict(spec: String, data: &'static str) -> String {
    std::thread::Builder::new()
        .stack_size(EVAL_STACK)
        .spawn(move || match parse(&spec) {
            Err(d) => d[0].title.clone(),
            Ok(s) => {
                let o = run_test(&s, data);
                match o.diags.first() {
                    Some(d) => d.title.clone(),
                    None => "pass".into(),
                }
            }
        })
        .unwrap()
        .join()
        .expect("no panic and no stack overflow")
}

const DATA: &str = "n: 3\nz: 1";
const TOO_DEEP: &str = "The formula is nested too deeply";

fn with_value(value: &str) -> String {
    format!("In: n∈N\nOut: z∈Z\nPre: -\nPost: n=n and z={value}")
}

fn with_condition(condition: &str) -> String {
    format!("In: n∈N\nOut: z∈Z\nPre: -\nPost: n=n and z=1 and {condition}")
}

#[test]
fn reasonable_depths_work() {
    let k = 40;
    let parens = format!("{}1{}", "(".repeat(k), ")".repeat(k));
    assert_eq!(verdict(with_value(&parens), DATA), "pass");
    let signs = format!("{}1", "-".repeat(k));
    assert_eq!(verdict(with_value(&signs), DATA), "pass");
    let nots = format!("{}true", "not ".repeat(k));
    assert_eq!(verdict(with_condition(&nots), DATA), "pass");
    let quantifiers = format!("{}(n=n)", "∀i∈[1..1]:".repeat(k));
    assert_eq!(verdict(with_condition(&quantifiers), DATA), "pass");
    let sum = vec!["1"; 300].join("+");
    assert_eq!(verdict(with_value(&sum), "n: 3\nz: 300"), "pass");
    let ands = vec!["n=n"; 300].join(" and ");
    assert_eq!(verdict(with_condition(&ands), DATA), "pass");
    let equal = vec!["n"; 10_000].join("=");
    assert_eq!(verdict(with_condition(&equal), DATA), "pass");
}

#[test]
fn deep_nesting_is_refused() {
    for k in [65, 1000, 50_000] {
        let nested = |open: &str, close: &str| {
            with_value(&format!("{}1{}", open.repeat(k), close.repeat(k)))
        };
        for spec in [
            nested("(", ")"),
            nested("[", "]"),
            nested("-", ""),
            nested("abs(", ")"),
            nested("n[", "]"),
            nested("SUM(i=1..1,", ")"),
            with_condition(&format!("{}true", "not ".repeat(k))),
            with_condition(&format!("{}(n=n)", "∀i∈[1..1]:".repeat(k))),
            with_condition(&format!("({})", vec!["n=n"; k].join(" -> "))),
        ] {
            assert_eq!(verdict(spec, DATA), TOO_DEEP, "{k} levels");
        }
    }
}

#[test]
fn endless_chains_are_refused() {
    for k in [500, 200_000] {
        for spec in [
            with_value(&vec!["1"; k].join("+")),
            with_value(&vec!["2"; k].join("*")),
            with_value(&format!("n{}", "[1]".repeat(k))),
            with_value(&format!("n{}", ".a".repeat(k))),
            with_value(&format!("n{}", "!".repeat(k))),
            with_condition(&vec!["n=n"; k].join(" and ")),
            with_condition(&vec!["n=n"; k].join(" or ")),
        ] {
            assert_eq!(verdict(spec, DATA), TOO_DEEP, "{k} parts");
        }
    }
}

#[test]
fn deep_types_are_refused() {
    for k in [65, 20_000] {
        let record = format!(
            "In: n∈{}N{}\nOut: z∈Z\nPre: -\nPost: z=1 and n=n",
            "(a:".repeat(k),
            ")".repeat(k)
        );
        assert_eq!(verdict(record, DATA), TOO_DEEP);
        let dimensions = format!(
            "In: n∈N{}\nOut: z∈Z\nPre: -\nPost: z=1 and n=n",
            "[1..2]".repeat(k)
        );
        assert_eq!(verdict(dimensions, DATA), TOO_DEEP);
    }
}

#[test]
fn deep_test_data_is_refused() {
    let k = 50_000;
    let spec = "In: n∈N[1..]\nOut: z∈Z\nPre: -\nPost: z=1 and n=n".to_string();
    let data: &'static str = format!("n: {}1{}\nz: 1", "[".repeat(k), "]".repeat(k)).leak();
    assert_ne!(verdict(spec, data), "pass");
}

#[test]
fn runaway_recursion_is_stopped() {
    let spec = |body: &str| {
        format!("In: n∈N\nOut: z∈Z\nFn: f:N->N, f(k)={body}\nPre: -\nPost: n=n and z=f(n)")
    };
    assert_eq!(verdict(spec("f(k-1)"), DATA), "`f` calls itself too deeply");
    assert_eq!(
        verdict(spec("k*f(k-1)+1"), DATA),
        "`f` calls itself too deeply"
    );
    let body = format!("f(k-1)+{}", vec!["1"; 300].join("+"));
    assert_eq!(
        verdict(spec(&body), DATA),
        "The evaluation is nested too deeply"
    );
    let mutual = "In: n∈N\nOut: z∈Z\nFn: f:N->N, f(k)=g(k)+1\nFn: g:N->N, g(k)=f(k)*2\nPre: -\nPost: n=n and z=f(n)";
    assert!(verdict(mutual.to_string(), DATA).ends_with("calls itself too deeply"));
}

#[test]
fn recursion_that_ends_is_evaluated() {
    let spec = "In: n∈N\nOut: z∈Z\nFn: third:Z->L, third(k)=(k=0 or (k>0 and third(k-3)))\nPre: -\nPost: z=1 and third(n) and not third(n+1) and third(900)";
    assert_eq!(verdict(spec.to_string(), DATA), "pass");
}
