//! Auto Stuki: structograms built from a specification.
//!
//! The postcondition is split at its `and`s, and each part that gives an output its value becomes
//! steps of the main structogram: `x=…` an assignment, the patterns (`SUM`, `COUNT`, `MAX`,
//! `SEARCH`, …) and the quantifiers the standard algorithms taught at ELTE, and
//! `∀i∈[a..b]:(y[i]=…)` a counting loop. Every `Fv:`/`Fn:` function gets a structogram of its own.
//! Parts that don't compute anything (conditions on the result) are kept as tinted statements, so
//! nothing of the specification silently disappears.
//!
//! The texts follow the language of the specification: `Be/Ki/Ef/Uf` gives Hungarian keywords
//! and types (`és`, `nem`, `Egész`), `In/Out/Pre/Post` English ones.

use super::model::{Case, Diagram, Kind, Node, Program, fresh_id};
use super::rich::Rich;
use crate::parser::{BinOp, Expr, ExprKind, Role, Spec, Type, macro_alias, vars_in};
use std::collections::HashSet;

/// Background of the statements that could not be turned into steps.
pub const UNTRANSLATED_BG: &str = "#ffcccc";

pub struct Generated {
    pub program: Program,
    /// Parts of the postcondition kept as plain (tinted) statements.
    pub untranslated: usize,
}

struct Words {
    and: &'static str,
    not: &'static str,
    yes: &'static str,
    no: &'static str,
    int: &'static str,
    real: &'static str,
    boolean: &'static str,
    string: &'static str,
    character: &'static str,
    array: &'static str,
    record: &'static str,
    main: &'static str,
    // names of the variables the algorithms need
    sum: &'static str,
    count: &'static str,
    found: &'static str,
    all: &'static str,
    ind: &'static str,
    max_ind: &'static str,
    max_val: &'static str,
    min_ind: &'static str,
    min_val: &'static str,
    items: &'static str,
}

const HU: Words = Words {
    and: "és",
    not: "nem",
    yes: "igaz",
    no: "hamis",
    int: "Egész",
    real: "Valós",
    boolean: "Logikai",
    string: "Szöveg",
    character: "Karakter",
    array: "Tömb",
    record: "Rekord",
    main: "Főprogram",
    sum: "s",
    count: "db",
    found: "van",
    all: "mind",
    ind: "ind",
    max_ind: "maxind",
    max_val: "maxért",
    min_ind: "minind",
    min_val: "minért",
    items: "y",
};

const EN: Words = Words {
    and: "and",
    not: "not",
    yes: "true",
    no: "false",
    int: "Integer",
    real: "Real",
    boolean: "Boolean",
    string: "String",
    character: "Character",
    array: "Array",
    record: "Record",
    main: "Main",
    sum: "s",
    count: "cnt",
    found: "found",
    all: "all",
    ind: "ind",
    max_ind: "maxind",
    max_val: "maxval",
    min_ind: "minind",
    min_val: "minval",
    items: "y",
};

/// The pattern's name as the evaluator knows it (`MEGSZÁMOL` → `COUNT`).
fn canonical(name: &str) -> &str {
    match name {
        "DARAB" => "COUNT",
        _ => macro_alias(name).unwrap_or(name),
    }
}

/// Patterns that give a single value (the others give a tuple or an array).
fn single_valued(name: &str) -> bool {
    matches!(
        canonical(name),
        "SUM" | "COUNT" | "EXISTS" | "FORALL" | "SELECT"
    )
}

fn node(kind: Kind) -> Node {
    Node {
        id: fresh_id(),
        bg: None,
        kind,
    }
}

fn stmt(text: impl AsRef<str>) -> Node {
    node(Kind::Statement(Rich::plain(text.as_ref())))
}

fn seq(nodes: Vec<Node>) -> Vec<Node> {
    if nodes.is_empty() {
        vec![Node::noop()]
    } else {
        nodes
    }
}

fn if_then(cond: impl AsRef<str>, yes: Vec<Node>) -> Node {
    node(Kind::If {
        cond: Rich::plain(cond.as_ref()),
        yes: seq(yes),
        no: vec![Node::noop()],
    })
}

fn while_loop(cond: impl AsRef<str>, body: Vec<Node>) -> Node {
    node(Kind::While {
        cond: Rich::plain(cond.as_ref()),
        body: seq(body),
    })
}

fn conjuncts(e: &Expr) -> Vec<&Expr> {
    match &*e.kind {
        ExprKind::Bin(BinOp::And, a, b) => {
            let mut v = conjuncts(a);
            v.extend(conjuncts(b));
            v
        }
        _ => vec![e],
    }
}

/// Needs no parentheses after `nem` or inside a longer expression.
fn atomic(e: &Expr) -> bool {
    matches!(
        &*e.kind,
        ExprKind::Var(_)
            | ExprKind::Int(_)
            | ExprKind::Real(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Call(..)
            | ExprKind::Index(..)
            | ExprKind::Field(..)
            | ExprKind::Tuple(_)
            | ExprKind::Array(..)
            | ExprKind::Macro { .. }
    )
}

/// Parentheses never close more than they opened (so `(a)+(b)` keeps its outer ones).
fn balanced(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// The loop variables of the patterns and quantifiers in `e`.
fn loop_vars(e: &Expr, out: &mut HashSet<String>) {
    if let ExprKind::Quant { var, .. } | ExprKind::Macro { var, .. } = &*e.kind {
        out.insert(var.clone());
    }
    e.children().into_iter().for_each(|c| loop_vars(c, out));
}

/// Line breaks (and the indentation after them) become single spaces.
fn one_line(s: &str) -> String {
    let mut out = String::new();
    let mut lines = s.lines().map(str::trim).filter(|l| !l.is_empty());
    if let Some(first) = lines.next() {
        out.push_str(first);
    }
    for l in lines {
        out.push(' ');
        out.push_str(l);
    }
    out
}

/// Has an operator outside brackets: `a+1` needs parentheses when put into `2*…`, `f(a)` doesn't.
fn needs_parens(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ if depth == 0 && !(c.is_alphanumeric() || c == '_' || c == '.') => return true,
            _ => {}
        }
    }
    false
}

fn contains_lift(e: &Expr) -> bool {
    match &*e.kind {
        ExprKind::Quant { .. } => true,
        ExprKind::Macro { name, .. } if single_valued(name) => true,
        _ => e.children().into_iter().any(contains_lift),
    }
}

/// `=` with a target on one side: (target, value).
fn equation(e: &Expr) -> Option<(&Expr, &Expr)> {
    match &*e.kind {
        ExprKind::Cmp(v, ops) if v.len() == 2 && ops[0] == "=" => Some((&v[0], &v[1])),
        _ => None,
    }
}

struct Gen<'a> {
    spec: &'a Spec,
    w: &'static Words,
    /// The names of the specification, so that new variables get names of their own.
    taken: HashSet<String>,
    /// Local declarations of the structogram being built: (name, type).
    locals: Vec<(String, String)>,
    /// Outputs that already have their value.
    assigned: HashSet<String>,
    untranslated: usize,
}

pub fn from_spec(spec: &Spec, title: &str) -> Generated {
    let mut g = Gen {
        spec,
        w: if spec.post_kw == "Uf:" { &HU } else { &EN },
        taken: HashSet::new(),
        locals: vec![],
        assigned: HashSet::new(),
        untranslated: 0,
    };
    g.taken.extend(spec.decls.iter().map(|d| d.name.clone()));
    g.taken.extend(spec.aliases.keys().cloned());
    for (name, f) in &spec.funcs {
        g.taken.insert(name.clone());
        g.taken.extend(f.params.iter().cloned());
        g.taken.extend(vars_in(&f.body));
    }
    g.taken.extend(vars_in(&spec.post));
    loop_vars(&spec.post, &mut g.taken);
    for f in spec.funcs.values() {
        loop_vars(&f.body, &mut g.taken);
    }
    let mut program = Program::default();
    let title = title.trim();
    let mut main = Diagram::new();
    main.name = Rich::plain(if title.is_empty() { g.w.main } else { title });
    main.show_return = false;
    main.body = seq(g.postcondition());
    g.finish_locals(&mut main);
    program.diagrams = vec![main];
    let mut funcs: Vec<_> = spec.funcs.iter().collect();
    funcs.sort_by_key(|(_, f)| f.body.start);
    for (name, f) in funcs {
        let d = g.function(name, f);
        program.diagrams.push(d);
    }
    Generated {
        program,
        untranslated: g.untranslated,
    }
}

impl Gen<'_> {
    /// The source of `e`, without parentheses around all of it (`∀i∈[1..n]:(T(i))` → `T(i)`).
    fn text(&self, e: &Expr) -> String {
        let t = one_line(self.spec.text(e));
        if matches!(&*e.kind, ExprKind::Tuple(_)) {
            return t;
        }
        match t.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            Some(inner) if balanced(inner) => inner.trim().to_string(),
            _ => t,
        }
    }

    fn fresh(&self, base: &str) -> String {
        (1..)
            .map(|n| {
                if n == 1 {
                    base.to_string()
                } else {
                    format!("{base}{n}")
                }
            })
            .find(|n| !self.taken.contains(n) && !self.locals.iter().any(|(l, _)| l == n))
            .unwrap()
    }

    /// Declare a local variable of the current structogram (once; never inputs or outputs).
    fn local(&mut self, name: &str, ty: &str) {
        if self.spec.decl(name).is_none() && !self.locals.iter().any(|(n, _)| n == name) {
            self.locals.push((name.to_string(), ty.to_string()));
        }
    }

    fn finish_locals(&mut self, d: &mut Diagram) {
        let locals = std::mem::take(&mut self.locals);
        d.show_variables = !locals.is_empty();
        let text = locals
            .iter()
            .map(|(n, t)| format!("{n}:{t}"))
            .collect::<Vec<_>>()
            .join(", ");
        d.variables = Rich::plain(&text);
    }

    fn type_text(&self, t: &Type) -> String {
        let w = self.w;
        match t {
            Type::Nat | Type::Int | Type::Interval(..) => w.int.into(),
            Type::Real => w.real.into(),
            Type::Bool => w.boolean.into(),
            Type::Str => w.string.into(),
            Type::Char => w.character.into(),
            Type::Array { lo, hi, elem } => format!(
                "{}[{}..{}:{}]",
                w.array,
                self.text(lo),
                hi.as_ref().map(|h| self.text(h)).unwrap_or_default(),
                self.type_text(elem)
            ),
            Type::Record(fields) => format!(
                "{}({})",
                w.record,
                fields
                    .iter()
                    .map(|(n, t)| format!("{n}:{}", self.type_text(t)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Named(n) => n.clone(),
        }
    }

    /// The type of a computed value, when nothing is declared for it: real if it involves reals.
    fn guess_type(&self, e: &Expr) -> String {
        fn real(spec: &Spec, t: &Type) -> bool {
            match spec.resolve(t) {
                Some(Type::Real) => true,
                Some(Type::Array { elem, .. }) => real(spec, elem),
                Some(Type::Record(f)) => f.iter().any(|(_, t)| real(spec, t)),
                _ => false,
            }
        }
        fn any_real(spec: &Spec, e: &Expr) -> bool {
            let here = match &*e.kind {
                ExprKind::Real(_) | ExprKind::Bin(BinOp::Div, ..) => true,
                ExprKind::Var(n) => spec.decl(n).is_some_and(|d| real(spec, &d.ty)),
                _ => false,
            };
            here || e.children().into_iter().any(|c| any_real(spec, c))
        }
        if any_real(self.spec, e) {
            self.w.real
        } else {
            self.w.int
        }
        .into()
    }

    /// `e` with `var` replaced by `with` (for the first element of MAX: `f(i)` → `f(1)`).
    fn subst(&self, e: &Expr, var: &str, with: &str) -> String {
        fn spans(e: &Expr, var: &str, out: &mut Vec<(usize, usize)>) {
            match &*e.kind {
                ExprKind::Var(n) if n == var => out.push((e.start, e.end)),
                // a nested loop with the same variable hides it
                ExprKind::Quant { var: v, .. } | ExprKind::Macro { var: v, .. } if v == var => {}
                _ => e.children().into_iter().for_each(|c| spans(c, var, out)),
            }
        }
        let mut found = vec![];
        spans(e, var, &mut found);
        let with = if needs_parens(with) {
            format!("({with})")
        } else {
            with.to_string()
        };
        self.splice(e, found.into_iter().map(|(a, b)| (a, b, with.clone())))
    }

    /// The source of `e` with some of its parts (byte spans, in order) replaced.
    fn splice(&self, e: &Expr, parts: impl IntoIterator<Item = (usize, usize, String)>) -> String {
        let src = &self.spec.src;
        let mut out = String::new();
        let mut at = e.start;
        for (a, b, with) in parts {
            out.push_str(src.get(at..a).unwrap_or(""));
            out.push_str(&with);
            at = b;
        }
        out.push_str(src.get(at..e.end).unwrap_or(""));
        one_line(&out)
    }

    fn negate(&self, e: &Expr) -> String {
        match &*e.kind {
            ExprKind::Not(x) => self.text(x),
            _ if atomic(e) => format!("{} {}", self.w.not, self.text(e)),
            _ => format!("{} ({})", self.w.not, self.text(e)),
        }
    }

    /// A condition joined to others with `és`: parenthesized if it holds an `or` or `->`.
    fn conj_part(&self, e: &Expr) -> String {
        match &*e.kind {
            ExprKind::Bin(BinOp::Or | BinOp::Imp, ..) => format!("({})", self.text(e)),
            _ => self.text(e),
        }
    }

    fn plus_one(&self, lo: &Expr) -> String {
        match &*lo.kind {
            ExprKind::Int(k) => (k + 1).to_string(),
            _ => format!("{}+1", self.text(lo)),
        }
    }

    /// The text of `e`'s value, after the steps (added to `out`) that compute the patterns and
    /// quantifiers inside it.
    fn value(&mut self, e: &Expr, out: &mut Vec<Node>) -> String {
        if !contains_lift(e) {
            return self.text(e);
        }
        match &*e.kind {
            ExprKind::Macro {
                name,
                var,
                lo,
                hi,
                args,
            } if single_valued(name) => {
                let t = self.temp(name, args);
                self.pattern(name, var, lo, hi.as_ref(), args, &[Some(t.clone())], out);
                t
            }
            ExprKind::Quant {
                all,
                var,
                lo,
                hi,
                body,
            } => {
                let name = if *all { "FORALL" } else { "EXISTS" };
                let t = self.temp(name, &[]);
                let args = std::slice::from_ref(body);
                self.pattern(name, var, lo, Some(hi), args, &[Some(t.clone())], out);
                t
            }
            _ => {
                // compute the outermost patterns first, then use their variables
                let mut parts = vec![];
                let mut pending = vec![e];
                let mut lifts = vec![];
                while let Some(x) = pending.pop() {
                    let lift = matches!(&*x.kind, ExprKind::Quant { .. })
                        || matches!(&*x.kind, ExprKind::Macro { name, .. } if single_valued(name));
                    if lift {
                        lifts.push(x);
                    } else {
                        let mut c = x.children();
                        c.reverse();
                        pending.extend(c);
                    }
                }
                lifts.sort_by_key(|x| x.start);
                for x in lifts {
                    let v = self.value(x, out);
                    parts.push((x.start, x.end, v));
                }
                self.splice(e, parts)
            }
        }
    }

    /// A new local variable for a pattern's single result.
    fn temp(&mut self, name: &str, args: &[Expr]) -> String {
        self.fresh_slots(name, args).remove(0)
    }

    /// The results of a pattern: (variable name to use, type), in the order of its tuple.
    fn slots(&self, name: &str, args: &[Expr]) -> Vec<(&'static str, String)> {
        let w = self.w;
        let guess = args
            .first()
            .map(|f| self.guess_type(f))
            .unwrap_or(w.int.into());
        let (bases, types): (&[&'static str], Vec<String>) = match canonical(name) {
            "SUM" => (&[w.sum], vec![guess]),
            "COUNT" => (&[w.count], vec![w.int.into()]),
            "EXISTS" => (&[w.found], vec![w.boolean.into()]),
            "FORALL" => (&[w.all], vec![w.boolean.into()]),
            "SELECT" => (&[w.ind], vec![w.int.into()]),
            "SEARCH" => (&[w.found, w.ind], vec![w.boolean.into(), w.int.into()]),
            "COPY" => (&[w.items], vec![w.array.into()]),
            "MIS" => (&[w.count, w.items], vec![w.int.into(), w.array.into()]),
            "MAX" => (&[w.max_ind, w.max_val], vec![w.int.into(), guess]),
            "MIN" => (&[w.min_ind, w.min_val], vec![w.int.into(), guess]),
            "CONDMAX" => (
                &[w.found, w.max_ind, w.max_val],
                vec![w.boolean.into(), w.int.into(), guess],
            ),
            _ => (
                &[w.found, w.min_ind, w.min_val],
                vec![w.boolean.into(), w.int.into(), guess],
            ),
        };
        bases.iter().copied().zip(types).collect()
    }

    /// New local variables for all the results of a pattern.
    fn fresh_slots(&mut self, name: &str, args: &[Expr]) -> Vec<String> {
        self.slots(name, args)
            .into_iter()
            .map(|(base, ty)| {
                let n = self.fresh(base);
                self.local(&n, &ty);
                n
            })
            .collect()
    }

    /// The steps of a pattern. `targets` holds the variables for its results (in the order of
    /// the tuple it gives; `None`: a slot left empty, which then gets a variable of its own).
    #[allow(clippy::too_many_arguments)]
    fn pattern(
        &mut self,
        name: &str,
        var: &str,
        lo: &Expr,
        hi: Option<&Expr>,
        args: &[Expr],
        targets: &[Option<String>],
        out: &mut Vec<Node>,
    ) {
        let w = self.w;
        let name = canonical(name);
        let t: Vec<String> = self
            .slots(name, args)
            .into_iter()
            .enumerate()
            .map(|(k, (base, ty))| match targets.get(k).cloned().flatten() {
                Some(n) => n,
                None => {
                    let n = self.fresh(base);
                    self.local(&n, &ty);
                    n
                }
            })
            .collect();
        self.local(var, w.int);
        let lo_t = self.text(lo);
        let hi_t = hi.map(|h| self.text(h)).unwrap_or_default();
        let range = format!("{var}={lo_t}..{hi_t}");
        let set = |x: &str, v: &str| stmt(format!("{x}:={v}"));
        let step = || set(var, &format!("{var}+1"));
        match name {
            "SUM" => {
                out.push(set(&t[0], "0"));
                let mut body = vec![];
                match args.get(1) {
                    Some(c) => {
                        let cv = self.value(c, &mut body);
                        let mut yes = vec![];
                        let fv = self.value(&args[0], &mut yes);
                        yes.push(set(&t[0], &format!("{}+{fv}", t[0])));
                        body.push(if_then(cv, yes));
                    }
                    None => {
                        let fv = self.value(&args[0], &mut body);
                        body.push(set(&t[0], &format!("{}+{fv}", t[0])));
                    }
                }
                out.push(while_loop(range, body));
            }
            "COUNT" => {
                out.push(set(&t[0], "0"));
                let mut body = vec![];
                let cv = self.value(&args[0], &mut body);
                body.push(if_then(cv, vec![set(&t[0], &format!("{}+1", t[0]))]));
                out.push(while_loop(range, body));
            }
            "EXISTS" | "FORALL" | "SEARCH" => {
                let c = &args[0];
                let cond = if name == "FORALL" {
                    self.conj_part(c)
                } else {
                    self.negate(c)
                };
                out.push(set(var, &lo_t));
                out.push(while_loop(
                    format!("{var}≤{hi_t} {} {cond}", w.and),
                    vec![step()],
                ));
                if name == "FORALL" {
                    out.push(set(&t[0], &format!("{var}>{hi_t}")));
                } else {
                    out.push(set(&t[0], &format!("{var}≤{hi_t}")));
                }
                if name == "SEARCH" {
                    out.push(if_then(&t[0], vec![set(&t[1], var)]));
                }
            }
            "SELECT" => {
                out.push(set(var, &lo_t));
                out.push(while_loop(self.negate(&args[0]), vec![step()]));
                out.push(set(&t[0], var));
            }
            "COPY" => {
                let mut body = vec![];
                let fv = self.value(&args[0], &mut body);
                body.push(set(&format!("{}[{var}]", t[0]), &fv));
                out.push(while_loop(range, body));
            }
            "MIS" => {
                out.push(set(&t[0], "0"));
                let mut body = vec![];
                let cv = self.value(&args[0], &mut body);
                let mut yes = vec![set(&t[0], &format!("{}+1", t[0]))];
                let item = match args.get(1) {
                    Some(f) => self.value(f, &mut yes),
                    None => var.to_string(),
                };
                yes.push(set(&format!("{}[{}]", t[1], t[0]), &item));
                body.push(if_then(cv, yes));
                out.push(while_loop(range, body));
            }
            "MAX" | "MIN" => {
                let rel = if name == "MAX" { ">" } else { "<" };
                out.push(set(&t[0], &lo_t));
                out.push(set(&t[1], &self.subst(&args[0], var, &lo_t)));
                let mut body = vec![];
                let fv = self.value(&args[0], &mut body);
                body.push(if_then(
                    format!("{fv}{rel}{}", t[1]),
                    vec![set(&t[0], var), set(&t[1], &fv)],
                ));
                out.push(while_loop(
                    format!("{var}={}..{hi_t}", self.plus_one(lo)),
                    body,
                ));
            }
            _ => {
                // CONDMAX / CONDMIN: the three cases of the ELTE algorithm
                let rel = if name == "CONDMAX" { ">" } else { "<" };
                let (f, c) = (&args[0], &args[1]);
                out.push(set(&t[0], w.no));
                let mut body = vec![];
                let fv = self.value(f, &mut body);
                let cond = self.conj_part(c);
                let found = &t[0];
                let case = |cond: String, body: Vec<Node>| Case {
                    id: fresh_id(),
                    cond: Some(Rich::plain(&cond)),
                    body: seq(body),
                };
                let cases = vec![
                    case(self.negate(c), vec![]),
                    case(
                        format!("{cond} {} {found}", w.and),
                        vec![if_then(
                            format!("{fv}{rel}{}", t[2]),
                            vec![set(&t[1], var), set(&t[2], &fv)],
                        )],
                    ),
                    case(
                        format!("{cond} {} {} {found}", w.and, w.not),
                        vec![set(found, w.yes), set(&t[1], var), set(&t[2], &fv)],
                    ),
                ];
                body.push(node(Kind::Switch { cases }));
                out.push(while_loop(range, body));
            }
        }
    }

    // ---------------------------------------------------------------- the postcondition

    /// An output (or auxiliary variable) that has no value yet.
    fn target(&self, e: &Expr) -> Option<String> {
        match &*e.kind {
            ExprKind::Var(n)
                if self
                    .spec
                    .decl(n)
                    .is_some_and(|d| d.role != Role::In && d.init.is_none())
                    && !self.assigned.contains(n) =>
            {
                Some(n.clone())
            }
            _ => None,
        }
    }

    /// `x` or `(x, , y)`: the variables an `=` gives values to.
    fn targets(&self, e: &Expr) -> Option<Vec<Option<String>>> {
        match &*e.kind {
            ExprKind::Tuple(slots) => {
                // every written slot must be a target; empty ones stay `None`
                let t = slots
                    .iter()
                    .map(|s| match s {
                        None => Some(None),
                        Some(s) => self.target(s).map(Some),
                    })
                    .collect::<Option<Vec<_>>>()?;
                t.iter().any(Option::is_some).then_some(t)
            }
            _ => Some(vec![Some(self.target(e)?)]),
        }
    }

    /// `∀i∈[a..b]:(y[i]=…)`: the element assignments, as (target, value).
    fn elementwise<'e>(&self, body: &'e Expr, var: &str) -> Option<Vec<(&'e Expr, &'e Expr)>> {
        conjuncts(body)
            .into_iter()
            .map(|c| {
                let (l, r) = equation(c)?;
                let is_elem = |e: &Expr| match &*e.kind {
                    ExprKind::Index(base, idx) => {
                        self.target(base).is_some()
                            && idx
                                .iter()
                                .any(|i| matches!(&*i.kind, ExprKind::Var(v) if v == var))
                    }
                    _ => false,
                };
                if is_elem(l) {
                    Some((l, r))
                } else if is_elem(r) {
                    Some((r, l))
                } else {
                    None
                }
            })
            .collect()
    }

    /// The outputs a part of the postcondition sets, and the ones it reads.
    fn sets_and_reads(&self, c: &Expr) -> (Vec<String>, Vec<String>) {
        if let ExprKind::Bin(BinOp::Imp, a, b) = &*c.kind {
            let (mut sets, mut reads) = (vec![], vec![]);
            for p in conjuncts(a).into_iter().chain(conjuncts(b)) {
                let (s, r) = self.sets_and_reads(p);
                sets.extend(s);
                reads.extend(r);
            }
            reads.retain(|r| !sets.contains(r));
            return (sets, reads);
        }
        let outputs = |e: &Expr| -> Vec<String> {
            vars_in(e)
                .into_iter()
                .filter(|n| self.spec.decl(n).is_some_and(|d| d.role != Role::In))
                .collect()
        };
        if let Some((l, r)) = equation(c) {
            for (t, v) in [(l, r), (r, l)] {
                if let Some(ts) = self.targets(t) {
                    return (ts.into_iter().flatten().collect(), outputs(v));
                }
            }
        }
        if let ExprKind::Quant {
            all: true,
            var,
            body,
            ..
        } = &*c.kind
            && let Some(pairs) = self.elementwise(body, var)
        {
            let sets: Vec<String> = pairs.iter().flat_map(|(t, _)| outputs(t)).collect();
            let reads = outputs(c)
                .into_iter()
                .filter(|n| !sets.contains(n))
                .collect();
            return (sets, reads);
        }
        (vec![], outputs(c))
    }

    fn postcondition(&mut self) -> Vec<Node> {
        let parts = conjuncts(&self.spec.post);
        let mut done = vec![false; parts.len()];
        let mut out = vec![];
        // the parts that set outputs, each once the outputs it reads have their values
        loop {
            let ready = (0..parts.len()).find(|&k| {
                let (sets, reads) = self.sets_and_reads(parts[k]);
                !done[k]
                    && !sets.is_empty()
                    && reads
                        .iter()
                        .all(|r| self.assigned.contains(r) || sets.contains(r))
            });
            let Some(k) = ready else { break };
            self.part(k, &parts, &mut done, &mut out);
        }
        // the rest in the order written: what still sets something, then the plain conditions
        for k in 0..parts.len() {
            if !done[k] {
                self.part(k, &parts, &mut done, &mut out);
            }
        }
        out
    }

    /// Part `k` of the postcondition. `c -> …` takes a `not c -> …` along as its else branch.
    fn part(&mut self, k: usize, parts: &[&Expr], done: &mut [bool], out: &mut Vec<Node>) {
        done[k] = true;
        let ExprKind::Bin(BinOp::Imp, a, b) = &*parts[k].kind else {
            return self.step(parts[k], out);
        };
        let other = (0..parts.len()).find(|&j| {
            !done[j]
                && matches!(&*parts[j].kind, ExprKind::Bin(BinOp::Imp, a2, _) if self.opposite(a, a2))
        });
        let otherwise = other.and_then(|j| {
            done[j] = true;
            match &*parts[j].kind {
                ExprKind::Bin(_, _, b2) => Some(b2),
                _ => None,
            }
        });
        self.conditional(a, b, otherwise, out);
    }

    /// `c` and `nem c` (either way round).
    fn opposite(&self, a: &Expr, b: &Expr) -> bool {
        match (&*a.kind, &*b.kind) {
            (ExprKind::Not(x), _) => self.text(x) == self.text(b),
            (_, ExprKind::Not(y)) => self.text(a) == self.text(y),
            _ => false,
        }
    }

    /// `c -> x=…`: a decision. Equations in `c` that set outputs are done first, as steps.
    fn conditional(&mut self, a: &Expr, b: &Expr, otherwise: Option<&Expr>, out: &mut Vec<Node>) {
        let mut conds = vec![];
        for p in conjuncts(a) {
            if self.sets_and_reads(p).0.is_empty() {
                conds.push(p);
            } else {
                self.step(p, out);
            }
        }
        // each branch may set the same outputs
        let before = self.assigned.clone();
        let mut yes = vec![];
        for p in conjuncts(b) {
            self.step(p, &mut yes);
        }
        let set_in_yes = std::mem::replace(&mut self.assigned, before);
        let mut no = vec![];
        for p in otherwise.map(conjuncts).unwrap_or_default() {
            self.step(p, &mut no);
        }
        self.assigned.extend(set_in_yes);
        if conds.is_empty() {
            out.extend(yes);
            out.extend(no);
            return;
        }
        let cond = conds
            .iter()
            .map(|c| self.conj_part(c))
            .collect::<Vec<_>>()
            .join(&format!(" {} ", self.w.and));
        out.push(node(Kind::If {
            cond: Rich::plain(&cond),
            yes: seq(yes),
            no: seq(no),
        }));
    }

    /// One part of the postcondition.
    fn step(&mut self, c: &Expr, out: &mut Vec<Node>) {
        if let Some((l, r)) = equation(c) {
            // `egyeb=egyeb`: only there to mention a variable
            if self.text(l) == self.text(r) {
                return;
            }
            for (t, v) in [(l, r), (r, l)] {
                let Some(ts) = self.targets(t) else { continue };
                self.assign(t, &ts, v, out);
                self.assigned.extend(ts.into_iter().flatten());
                return;
            }
        }
        if let ExprKind::Quant {
            all: true,
            var,
            lo,
            hi,
            body,
        } = &*c.kind
            && let Some(pairs) = self.elementwise(body, var)
        {
            self.local(var, self.w.int);
            let mut steps = vec![];
            let mut sets = vec![];
            for (t, v) in pairs {
                let v = self.value(v, &mut steps);
                steps.push(stmt(format!("{}:={v}", self.text(t))));
                if let ExprKind::Index(base, _) = &*t.kind
                    && let Some(n) = self.target(base)
                {
                    sets.push(n);
                }
            }
            out.push(while_loop(
                format!("{var}={}..{}", self.text(lo), self.text(hi)),
                steps,
            ));
            self.assigned.extend(sets);
            return;
        }
        let mut n = stmt(self.text(c));
        n.bg = Some(UNTRANSLATED_BG.into());
        out.push(n);
        self.untranslated += 1;
    }

    fn assign(&mut self, t: &Expr, ts: &[Option<String>], v: &Expr, out: &mut Vec<Node>) {
        if let ExprKind::Macro {
            name,
            var,
            lo,
            hi,
            args,
        } = &*v.kind
        {
            return self.pattern(name, var, lo, hi.as_ref(), args, ts, out);
        }
        if let (
            ExprKind::Quant {
                all,
                var,
                lo,
                hi,
                body,
            },
            [Some(_)],
        ) = (&*v.kind, ts)
        {
            let name = if *all { "FORALL" } else { "EXISTS" };
            let args = std::slice::from_ref(body);
            return self.pattern(name, var, lo, Some(hi), args, ts, out);
        }
        let v = self.value(v, out);
        out.push(stmt(format!("{}:={v}", self.text(t))));
    }

    // ---------------------------------------------------------------- functions

    fn function(&mut self, name: &str, f: &crate::parser::Func) -> Diagram {
        let params = f
            .params
            .iter()
            .enumerate()
            .map(|(k, p)| match f.dom.get(k) {
                Some(t) if f.dom.len() == f.params.len() => format!("{p}:{}", self.type_text(t)),
                _ => p.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let cod = f
            .cod
            .iter()
            .map(|t| self.type_text(t))
            .collect::<Vec<_>>()
            .join(" x ");
        let mut d = Diagram::new();
        d.name = Rich::plain(&format!("{name}({params}):{cod}"));
        d.show_header = true;
        d.show_return = true;
        let mut body = vec![];
        let ret = match &*f.body.kind {
            ExprKind::Macro {
                name: m,
                var,
                lo,
                hi,
                args,
            } if !single_valued(m) => {
                // tuple results: every slot gets a variable, the function returns all of them
                let ts = self.fresh_slots(m, args);
                let targets: Vec<_> = ts.iter().cloned().map(Some).collect();
                self.pattern(m, var, lo, hi.as_ref(), args, &targets, &mut body);
                format!("({})", ts.join(", "))
            }
            _ => self.value(&f.body, &mut body),
        };
        d.ret = Rich::plain(&ret);
        d.body = seq(body);
        self.finish_locals(&mut d);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// The structograms as indented text: `[while i=1..n]`, `[if c]` … `[else]`, `[case c]`.
    fn outline(p: &Program) -> String {
        fn block(nodes: &[Node], depth: usize, out: &mut String) {
            let pad = "  ".repeat(depth);
            for n in nodes {
                match &n.kind {
                    Kind::Noop => out.push_str(&format!("{pad}-\n")),
                    Kind::Statement(t) => {
                        let mark = if n.bg.is_some() { " (!)" } else { "" };
                        out.push_str(&format!("{pad}{}{mark}\n", t.text));
                    }
                    Kind::If { cond, yes, no } => {
                        out.push_str(&format!("{pad}[if {}]\n", cond.text));
                        block(yes, depth + 1, out);
                        if !(no.len() == 1 && no[0].is_noop()) {
                            out.push_str(&format!("{pad}[else]\n"));
                            block(no, depth + 1, out);
                        }
                    }
                    Kind::While { cond, body } => {
                        out.push_str(&format!("{pad}[while {}]\n", cond.text));
                        block(body, depth + 1, out);
                    }
                    Kind::Repeat { cond, body } => {
                        out.push_str(&format!("{pad}[repeat {}]\n", cond.text));
                        block(body, depth + 1, out);
                    }
                    Kind::Switch { cases } => {
                        for c in cases {
                            let cond = c.cond.as_ref().map_or("else", |c| &c.text);
                            out.push_str(&format!("{pad}[case {cond}]\n"));
                            block(&c.body, depth + 1, out);
                        }
                    }
                }
            }
        }
        let mut s = String::new();
        for d in &p.diagrams {
            s.push_str(&format!("== {}\n", d.name.text));
            if d.show_variables {
                s.push_str(&format!("vars {}\n", d.variables.text));
            }
            block(&d.body, 0, &mut s);
            if d.show_header && d.show_return {
                s.push_str(&format!("return {}\n", d.ret.text));
            }
        }
        s
    }

    fn outline_of(spec: &str) -> String {
        let spec = parse(spec).unwrap_or_else(|e| panic!("{}", e[0].title));
        outline(&from_spec(&spec, "T").program)
    }

    #[test]
    fn count() {
        let out = outline_of(
            "Be:\nn∈N,\nszamok∈N[1..n]\nKi:\ndb∈N\nEf:\n-\nUf:\ndb=DARAB(i=1..n, szamok[i] mod 2 = 1)\n",
        );
        assert_eq!(
            out,
            "== T\nvars i:Egész\ndb:=0\n[while i=1..n]\n  [if szamok[i] mod 2 = 1]\n    db:=db+1\n"
        );
    }

    #[test]
    fn sum_with_condition_and_english_words() {
        let out = outline_of(
            "In:\nn∈N,\nx∈R[1..n]\nOut:\ns∈R\nPre:\n-\nPost:\ns=SUM(i=1..n, x[i], x[i]>0)\n",
        );
        assert_eq!(
            out,
            "== T\nvars i:Integer\ns:=0\n[while i=1..n]\n  [if x[i]>0]\n    s:=s+x[i]\n"
        );
    }

    #[test]
    fn max_and_search() {
        let out = outline_of(
            "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nmi∈N,\nme∈Z,\nvan∈L,\nind∈N\nEf:\nn>0\nUf:\n(mi,me)=MAX(i=1..n, x[i]) és (van,ind)=KERES(j=1..n, x[j]<0)\n",
        );
        assert_eq!(
            out,
            "== T\nvars i:Egész, j:Egész\n\
             mi:=1\nme:=x[1]\n[while i=2..n]\n  [if x[i]>me]\n    mi:=i\n    me:=x[i]\n\
             j:=1\n[while j≤n és nem (x[j]<0)]\n  j:=j+1\nvan:=j≤n\n[if van]\n  ind:=j\n"
        );
    }

    #[test]
    fn empty_slot_gets_a_local_and_order_follows_dependencies() {
        let out = outline_of(
            "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nme∈Z,\nk∈Z\nEf:\nn>0\nUf:\nk=me*2 és (,me)=MAX(i=1..n, x[i])\n",
        );
        assert!(
            out.starts_with("== T\nvars maxind:Egész, i:Egész\nmaxind:=1\nme:=x[1]\n"),
            "{out}"
        );
        assert!(out.ends_with("k:=me*2\n"), "{out}");
    }

    #[test]
    fn nested_pattern_and_elementwise_loop() {
        let out = outline_of(
            "Be:\nn∈N,\nm∈N,\nt∈Z[1..n,1..m]\nKi:\ny∈N[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:(y[i]=DARAB(j=1..m, t[i,j]>0))\n",
        );
        assert_eq!(
            out,
            "== T\nvars i:Egész, db:Egész, j:Egész\n[while i=1..n]\n  db:=0\n  [while j=1..m]\n    [if t[i,j]>0]\n      db:=db+1\n  y[i]:=db\n"
        );
    }

    #[test]
    fn functions_get_their_own_structogram() {
        let out = outline_of(
            "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nvan∈L\nFv:\njo:N->L, jo(i)=x[i]>0 és x[i] mod 2=0\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(jo(i))\n",
        );
        assert_eq!(
            out,
            "== T\nvars i:Egész\ni:=1\n[while i≤n és nem jo(i)]\n  i:=i+1\nvan:=i≤n\n\
             == jo(i:Egész):Logikai\n-\nreturn x[i]>0 és x[i] mod 2=0\n"
        );
    }

    #[test]
    fn condmax_and_untranslated_parts() {
        let spec = parse(
            "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nvan∈L,\nmi∈N,\nme∈Z\nEf:\n-\nUf:\n(van,mi,me)=MAXHA(i=1..n, x[i], x[i]<0) és me<0\n",
        )
        .unwrap();
        let g = from_spec(&spec, "");
        assert_eq!(g.untranslated, 1);
        let out = outline(&g.program);
        assert_eq!(
            out,
            "== Főprogram\nvars i:Egész\nvan:=hamis\n[while i=1..n]\n\
             [case nem (x[i]<0)]\n    -\n\
             [case x[i]<0 és van]\n    [if x[i]>me]\n      mi:=i\n      me:=x[i]\n\
             [case x[i]<0 és nem van]\n    van:=igaz\n    mi:=i\n    me:=x[i]\n\
             me<0 (!)\n"
                .replace("[case", "  [case")
        );
    }

    #[test]
    fn implications_become_one_decision() {
        let out = outline_of(
            "Be:\nn∈N,\nar∈N[1..n],\nk∈N\nSa:\nvan∈L,\nind∈N\nKi:\nsorszam∈Z\nEf:\n-\nUf:\n\
             (van -> sorszam=ind) és (nem van -> sorszam=-1) és\n(van,ind)=KERES(i=1..n, ar[i]<k) és k=k\n",
        );
        assert_eq!(
            out,
            "== T\nvars i:Egész\ni:=1\n[while i≤n és nem (ar[i]<k)]\n  i:=i+1\nvan:=i≤n\n\
             [if van]\n  ind:=i\n[if van]\n  sorszam:=ind\n[else]\n  sorszam:=-1\n"
        );
        // a search inside the condition comes first; the rest of the condition decides
        let out = outline_of(
            "In:\nn∈N,\nx∈N[1..n]\nAux:\nvan∈L,\nind∈N\nOut:\nlib∈N\nPre:\n-\nPost:\n\
             (van,ind)=SEARCH(i=1..n-1, x[i]<x[i+1]) and\n      van -> lib=x[ind]\n",
        );
        assert!(
            out.ends_with("[if van]\n  ind:=i\n[if van]\n  lib:=x[ind]\n"),
            "{out}"
        );
    }

    #[test]
    fn names_are_unique_per_structogram() {
        let out = outline_of(
            "In:\nn∈N,\nm∈N,\nt∈N[1..n,1..m]\nOut:\nmx∈N\nFn:\nmc:N->N x N, mc(i)=MAX(j=1..m, t[i,j])\nPre:\n-\nPost:\n(mx,)=MAX(i=1..n, mc(i).maxval)\n",
        );
        assert!(
            out.contains("== mc(i:Integer):Integer x Integer\nvars maxind:Integer, maxval:Integer"),
            "{out}"
        );
        assert!(out.contains("maxval:=mc(1).maxval\n"), "{out}");
    }

    #[test]
    fn sum_in_a_function() {
        let out = outline_of(
            "Be:\nn∈N,\nx∈R[1..n]\nKi:\natl∈R\nFv:\nossz:N->R, ossz(k)=SZUMMA(i=1..k, x[i])\nEf:\nn>0\nUf:\natl=ossz(n)/n\n",
        );
        assert_eq!(
            out,
            "== T\natl:=ossz(n)/n\n\
             == ossz(k:Egész):Valós\nvars s:Valós, i:Egész\ns:=0\n[while i=1..k]\n  s:=s+x[i]\nreturn s\n"
        );
    }
}
