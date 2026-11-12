//! Auto Stuki: structograms built from a specification.
//!
//! The postcondition is split at its `and`s, and each part that gives an output its value becomes
//! steps of the main structogram: `x=…` an assignment, the patterns (`SUM`, `COUNT`, `MAX`,
//! `SEARCH`, …) and the quantifiers the standard algorithms taught at ELTE, and
//! `∀i∈[a..b]:(y[i]=…)` a counting loop. Every `Fv:`/`Fn:` function gets a structogram of its own.
//! Parts that don't compute anything (conditions on the result) need no steps and are left out;
//! the outputs that still get no value are listed in [`Generated::unset`].
//!
//! The texts follow the language of the specification: `Be/Ki/Ef/Uf` gives Hungarian keywords
//! and types (`és`, `nem`, `Egész`), `In/Out/Pre/Post` English ones.

use super::model::{Case, Diagram, Kind, Node, Program, fresh_id};
use super::rich::Rich;
use crate::eval::result_names;
use crate::parser::{BinOp, Expr, ExprKind, Role, Spec, Type, macro_alias, vars_in};
use std::collections::HashSet;

mod helpers;
mod pattern;
mod post;
#[cfg(test)]
mod tests;

use helpers::*;

pub struct Generated {
    pub program: Program,
    /// Outputs no step gives a value (the postcondition only describes them).
    pub unset: Vec<String>,
}

struct Words {
    and: &'static str,
    or: &'static str,
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
    /// The statements that show the inputs at the start and the outputs at the end: `Be: n, x`,
    /// `Ki: s, db`.
    input: &'static str,
    out: &'static str,
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
    or: "vagy",
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
    input: "Be:",
    out: "Ki:",
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
    or: "or",
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
    input: "In:",
    out: "Out:",
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

struct Gen<'a> {
    spec: &'a Spec,
    w: &'static Words,
    /// The names of the specification, so that new variables get names of their own.
    taken: HashSet<String>,
    /// Local declarations of the structogram being built: (name, type).
    locals: Vec<(String, String)>,
    /// Outputs that already have their value.
    assigned: HashSet<String>,
    /// Arrays given their value element by element (`y[1]=…`, `∀i∈[2..n]:(y[i]=…)`).
    elems: HashSet<String>,
    active: Vec<String>,
    renames: Vec<(String, String)>,
}

pub fn from_spec(spec: &Spec, title: &str) -> Generated {
    let mut g = Gen {
        spec,
        w: if spec.post_kw == "Uf:" { &HU } else { &EN },
        taken: HashSet::new(),
        locals: vec![],
        assigned: HashSet::new(),
        elems: HashSet::new(),
        active: vec![],
        renames: vec![],
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
    // `Be: n, x` … `Ki: s, db`: the variables the program reads and the ones it shows
    let shown = |role: Role, word: &str| {
        let names: Vec<&str> = spec
            .decls
            .iter()
            .filter(|d| d.role == role)
            .map(|d| d.name.as_str())
            .collect();
        (!names.is_empty()).then(|| stmt(format!("{word} {}", names.join(", "))))
    };
    let mut body: Vec<Node> = shown(Role::In, g.w.input).into_iter().collect();
    body.extend(g.block(&conjuncts(&spec.post)));
    body.extend(shown(Role::Out, g.w.out));
    main.body = seq(body);
    g.finish_locals(&mut main);
    let unset = spec
        .decls
        .iter()
        .filter(|d| d.role == Role::Out && d.init.is_none() && !g.assigned.contains(&d.name))
        .map(|d| d.name.clone())
        .collect();
    program.diagrams = vec![main];
    let mut funcs: Vec<_> = spec.funcs.iter().collect();
    funcs.sort_by_key(|(_, f)| f.body.start);
    for (name, f) in funcs {
        let d = g.function(name, f);
        program.diagrams.push(d);
    }
    Generated { program, unset }
}

impl Gen<'_> {
    /// The source of `e`, without parentheses around all of it (`∀i∈[1..n]:(T(i))` → `T(i)`).
    fn text(&self, e: &Expr) -> String {
        self.bare(e, self.render(e, &self.renames, vec![]))
    }

    fn bare(&self, e: &Expr, t: String) -> String {
        if matches!(&*e.kind, ExprKind::Tuple(_)) {
            return t;
        }
        match t.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            Some(inner) if balanced(inner) => inner.trim().to_string(),
            _ => t,
        }
    }

    fn render(
        &self,
        e: &Expr,
        renames: &[(String, String)],
        fixed: Vec<(usize, usize, String)>,
    ) -> String {
        let mut parts = vec![];
        if !renames.is_empty() {
            renamed(e, renames, &mut vec![], &mut parts);
            parts.retain(|(a, b, _)| !fixed.iter().any(|(s, t, _)| s <= a && b <= t));
        }
        parts.extend(fixed);
        parts.sort_by_key(|p| p.0);
        self.splice(e, parts).trim().to_string()
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
        let with = if needs_parens(with) {
            format!("({with})")
        } else {
            with.to_string()
        };
        let mut renames = self.renames.clone();
        renames.push((var.to_string(), with));
        self.render(e, &renames, vec![])
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

    fn cond_value(&mut self, e: &Expr, out: &mut Vec<Node>) -> String {
        if !contains_lift(e) {
            return self.conj_part(e);
        }
        let v = self.value(e, out);
        match &*e.kind {
            ExprKind::Bin(BinOp::Or | BinOp::Imp, ..) => format!("({v})"),
            _ => v,
        }
    }

    fn shifted(&self, lo: &Expr, by: i64) -> String {
        match &*lo.kind {
            ExprKind::Int(k) => (k + by).to_string(),
            _ if by < 0 => format!("{}-{}", self.text(lo), -by),
            _ => format!("{}+{by}", self.text(lo)),
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
            _ if let Some((m, k)) = projection(e) => {
                let ExprKind::Macro {
                    name,
                    var,
                    lo,
                    hi,
                    args,
                } = &*m.kind
                else {
                    unreachable!()
                };
                let t = self.fresh_slots(name, args);
                let targets: Vec<_> = t.iter().cloned().map(Some).collect();
                self.pattern(name, var, lo, hi.as_ref(), args, &targets, out);
                t[k].clone()
            }
            _ => {
                // compute the outermost patterns first, then use their variables
                let mut parts = vec![];
                let mut pending = vec![e];
                let mut lifts = vec![];
                while let Some(x) = pending.pop() {
                    if lifted(x) {
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
                self.bare(e, self.render(e, &self.renames, parts))
            }
        }
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
        self.active = f.params.clone();
        let whole = matches!(&*f.body.kind, ExprKind::Macro { .. });
        let unclear = if whole {
            f.body.children().into_iter().any(|c| stray(c, false))
        } else {
            stray(&f.body, false)
        };
        let ret = match &*f.body.kind {
            // no steps for it: the function returns the body as written
            _ if unclear => self.text(&f.body),
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
        self.active.clear();
        d.ret = Rich::plain(&ret);
        d.body = seq(body);
        self.finish_locals(&mut d);
        d
    }
}
