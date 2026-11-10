//! Whatever is typed, the parser and the evaluator answer with a verdict or a message: they
//! never panic. Valid specifications and test cases are damaged in many small ways (the states
//! a text goes through while it is being typed) and run.

use speci_local::{parse, run_test, scaffold, trace_test};
use std::sync::atomic::AtomicBool;

const CORPUS: &[(&str, &str)] = &[
    (
        "In: n∈N, nums∈Z[1..n]\nOut: ind∈N\nPre: ∃i∈[1..n]:(nums[i] mod 2 = 1)\nPost: ind=SELECT(i>=1, nums[i] mod 2 = 1) and n=n",
        "n: 3\nnums: [2, 4, 7]\nind: 3",
    ),
    (
        "In: n∈N, pl∈Tpl[1..n], Tpl=(name:S x x:N x y:N)\nAux: van∈L, ind∈N\nOut: lib∈S\nPre: ∀i∈[1..n]: (pl[i].x≥0 and pl[i].y≥0)\nPost: (van,ind)=SEARCH(i=1..n-1,pl[i].y<pl[i+1].y) and\n      van -> lib=pl[ind].name // é",
        "n: 2\npl: [{name: a, x: 1, y: 2}, {name: \"b c\", x: 3, y: 4}]\nvan: true\nind: 1\nlib: a",
    ),
    (
        "In: n∈N, m∈N, map∈N[1..n,1..m]\nOut: mpx∈N, mpy∈N\nFn: maxcol: N->N x N, maxcol(i)=MAX(j=1..m,map[i,j])\nPre: ∀i∈[1..n]: (∀j∈[1..m]:(map[i,j]≥0))\nPost: (mpx,)=MAX(i=1..n,maxcol(i).maxval) and\n      mpy=maxcol(mpx).maxind",
        "n: 2\nm: 2\nmap: [[1, 2], [5, 4]]\nmpx: 2\nmpy: 1",
    ),
    (
        "In: n∈N, d∈N, tr∈N[1..n]\nOut: cnt∈N, days∈N[1..cnt]\nFn: total: N->N, total(i)=SUM(j=i-d..i-1,tr[j])\nPre: n>d>0\nPost: (cnt,days)=MIS(i=d+1..n,total(i)<tr[i],i)",
        "n: 4\nd: 1\ntr: [1, 2, 1, 5]\ncnt: 2\ndays: [2, 4]",
    ),
    (
        "Be:\nn∈N,\nk∈N,\nar∈N[1..n]\n\nSa:\nvan∈L,\nind∈N\n\nKi:\nsorszam∈Z,\ndb∈N\n\nFv:\nkisebb:N->L,\nkisebb(i)=ar[i]<k\n\nEf:\n1<=n<=100 és ∀i∈[1..n]:(1<=ar[i])\n\nUf:\n(van,ind)=SEARCH(i=1..n, kisebb(i)) and\n(van -> sorszam=ind) and\n(not van -> sorszam=-1) and\ndb=DARAB(i=1..n, ar[i]<k) /* ő */",
        "n: 3\nk: 2\nar: [5, 1, 7]\nvan: true\nind: 2\nsorszam: 2\ndb: 1",
    ),
    (
        "In: n∈N, nums∈Z[1..n], a∈N, s∈S, r∈R\nAux: van∈L\nOut: ind∈N, abs∈Z[1..n]\nPre: a∈[2..7] and 2≤a≤7 and not(true -> false) and (3, 5).1=3 and \"x\"+s≠'q' and r/2<a! and 2|a\nPost: (van,-ind)=SEARCH(i=-n..-1, nums[-i] mod 2 = 1) and abs=COPY(i=1..n, abs(nums[i])) and length(nums)=n and round(sqrt(r)) div 2 ≥ 0 and a=a and s=s",
        "n: 2\nnums: [2, 3]\na: 4\ns: hello\nr: 2.5\nvan: true\nind: 2\nabs: [2, 3]",
    ),
    (
        "In: n∈N\nOut: c∈L, y∈N[0..], z∈N\nFn: f:N->N, f(k)=k*f(k-1)+1\nPre: -\nPost: c=([[1, 2], [3, 4]]=[1..,1..:[1, 2], [3, 4]]) and y=[0..:5, n] and (EXISTS(i=1..n, i=2) or FORALL(i=1..n, i∤7)) and (,,z)=CONDMIN(i=1..n, i*i, i>1)",
        "n: 3\nc: true\ny: [5, 3]\nz: 4",
    ),
];

const PIECES: &[&str] = &[
    "(",
    ")",
    "[",
    "]",
    ",",
    ":",
    "..",
    "=",
    "->",
    "∈",
    "∀",
    "∃",
    "-",
    "+",
    "*",
    "/",
    "!",
    ".",
    "\"",
    "'",
    "„",
    "”",
    "//",
    "/*",
    "*/",
    "\n",
    " ",
    "and",
    "or",
    "not",
    "x",
    "n",
    "i",
    "1",
    "0",
    "9223372036854775807",
    "-9223372036854775808",
    "1e999",
    "0.5",
    "SUM(",
    "MAX(i=1..n,",
    "SELECT(i>=",
    "f(",
    "length(",
    "Fn:",
    "Post:",
    "In:",
    "é",
    "\u{0}",
    "\t",
    "{",
    "}",
    "- ",
    "true",
    "~",
    "[[",
    "]]",
    "1..",
    "mod",
    "div",
    "∤",
    "|",
    "≥",
    "≠",
];

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

fn damage(text: &str, rng: &mut Rng) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    for _ in 0..=rng.below(3) {
        let at = rng.below(chars.len() + 1);
        match rng.below(5) {
            0 if at < chars.len() => {
                chars.remove(at);
            }
            1 => {
                let end = (at + rng.below(12)).min(chars.len());
                chars.drain(at..end);
            }
            2 => chars.truncate(at),
            3 => {
                let end = (at + rng.below(20)).min(chars.len());
                let copy: Vec<char> = chars[at..end].to_vec();
                chars.splice(at..at, copy);
            }
            _ => {
                let piece = PIECES[rng.below(PIECES.len())];
                chars.splice(at..at, piece.chars());
            }
        }
    }
    chars.into_iter().collect()
}

fn exercise(spec: &str, data: &str) {
    let parsed = match parse(spec) {
        Ok(s) => s,
        Err(diags) => {
            assert!(!diags.is_empty(), "rejected without a message");
            for d in &diags {
                let _ = d.render(spec, data);
            }
            return;
        }
    };
    let outcome = run_test(&parsed, data);
    assert!(
        outcome.pass || !outcome.diags.is_empty(),
        "failed without a message"
    );
    for d in &outcome.diags {
        let _ = d.render(spec, data);
    }
    let _ = trace_test(&parsed, data, &AtomicBool::new(false));
    let _ = scaffold(&parsed, data);
    // Auto Stuki works on anything the parser accepts
    let built = speci_local::stuki::from_spec::from_spec(&parsed, "fuzz");
    let verdicts = speci_local::stuki::run::run_tests(
        &built.program,
        &parsed,
        &[data.to_string()],
        &AtomicBool::new(false),
    );
    assert!(
        verdicts[0].pass || !verdicts[0].message.is_empty(),
        "the structogram failed without a message"
    );
}

fn on_eval_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(speci_local::eval::EVAL_STACK)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e));
}

fn rounds() -> usize {
    std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(300)
}

#[test]
fn the_corpus_itself_is_valid() {
    for (spec, data) in CORPUS {
        let s = parse(spec).unwrap_or_else(|d| panic!("{spec}\n{d:?}"));
        let o = run_test(&s, data);
        assert!(o.pass, "{spec}\n{data}\n{:?}", o.diags);
    }
}

#[test]
fn damaged_documents_never_panic() {
    on_eval_stack(|| {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for round in 0..rounds() {
            for (spec, data) in CORPUS {
                let (s, d) = match round % 3 {
                    0 => (damage(spec, &mut rng), data.to_string()),
                    1 => (spec.to_string(), damage(data, &mut rng)),
                    _ => (damage(spec, &mut rng), damage(data, &mut rng)),
                };
                let shown = format!("--- specification\n{s}\n--- test case\n{d}");
                if std::panic::catch_unwind(|| exercise(&s, &d)).is_err() {
                    panic!("panicked on:\n{shown}");
                }
            }
        }
    });
}

#[test]
fn every_prefix_of_a_document_is_handled() {
    on_eval_stack(|| {
        for (spec, data) in CORPUS {
            let ends = spec.char_indices().map(|(i, _)| i).chain([spec.len()]);
            for end in ends {
                exercise(&spec[..end], data);
            }
            let ends = data.char_indices().map(|(i, _)| i).chain([data.len()]);
            for end in ends {
                exercise(spec, &data[..end]);
            }
        }
    });
}
