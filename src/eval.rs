//! Values and the expression / theorem-macro evaluator.

use crate::diag::Diagnostic;
use crate::parser::{BinOp, Expr, ExprKind, Spec};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Real(f64),
    Bool(bool),
    Str(String),
    Arr {
        from: i64,
        items: Vec<Value>,
    },
    Rec(Vec<(String, Value)>),
    Tuple(Vec<Value>, &'static [&'static str]),
    /// Result slot with no meaning (index of a failed SEARCH); equal to anything.
    Undef,
}

pub type Env = HashMap<String, Value>;

impl Value {
    pub fn eq_val(&self, o: &Value) -> bool {
        use Value::*;
        match (self, o) {
            (Undef, _) | (_, Undef) => true,
            (Int(a), Int(b)) => a == b,
            (Int(_) | Real(_), Int(_) | Real(_)) => (self.num() - o.num()).abs() < 1e-9,
            (Bool(a), Bool(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Arr { items: a, .. }, Arr { items: b, .. }) | (Tuple(a, _), Tuple(b, _)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_val(y))
            }
            (Rec(a), Rec(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && v.eq_val(v2)))
            }
            _ => false,
        }
    }
    fn num(&self) -> f64 {
        match self {
            Value::Int(i) => *i as f64,
            Value::Real(r) => *r,
            _ => f64::NAN,
        }
    }
    fn is_num(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Real(_))
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "a whole number",
            Value::Real(_) => "a real number",
            Value::Bool(_) => "a logical value",
            Value::Str(_) => "a text",
            Value::Arr { .. } => "an array",
            Value::Rec(_) => "a record",
            Value::Tuple(..) => "a tuple",
            Value::Undef => "undefined",
        }
    }
    /// Short rendering for messages: long arrays are abbreviated.
    pub fn short(&self) -> String {
        match self {
            Value::Arr { items, .. } if items.len() > 12 => format!("[{} elements]", items.len()),
            _ => self.to_string(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let list = |f: &mut fmt::Formatter, v: &[Value]| -> fmt::Result {
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{x}")?;
            }
            Ok(())
        };
        match self {
            Value::Int(i) => write!(f, "{i}"),
            Value::Real(r) => write!(f, "{r}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Arr { items, .. } => {
                write!(f, "[")?;
                list(f, items)?;
                write!(f, "]")
            }
            Value::Tuple(items, _) => {
                write!(f, "(")?;
                list(f, items)?;
                write!(f, ")")
            }
            Value::Rec(fs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fs.iter().enumerate() {
                    write!(f, "{}{k}: {v}", if i > 0 { ", " } else { "" })?;
                }
                write!(f, "}}")
            }
            Value::Undef => write!(f, "undefined"),
        }
    }
}

type R<T> = Result<T, Diagnostic>;

const MAX_STEPS: u64 = 50_000_000;
/// Longest loop a quantifier or theorem macro may run (also bounds memory for COPY/MIS).
const MAX_RANGE: i64 = 10_000_000;

pub struct Eval<'a> {
    pub spec: &'a Spec,
    depth: usize,
    steps: u64,
    /// Variables bound by `=` so far, so failed branches can be undone without cloning the environment.
    log: Vec<String>,
    cancel: Option<&'a AtomicBool>,
}

impl<'a> Eval<'a> {
    pub fn new(spec: &'a Spec) -> Self {
        Eval {
            spec,
            depth: 0,
            steps: 0,
            log: Vec::new(),
            cancel: None,
        }
    }

    /// Evaluation stops with an error as soon as `cancel` becomes true.
    pub fn with_cancel(spec: &'a Spec, cancel: &'a AtomicBool) -> Self {
        Eval {
            cancel: Some(cancel),
            ..Self::new(spec)
        }
    }

    /// Undo every binding made after `mark`.
    fn rollback(&mut self, mark: usize, env: &mut Env) {
        for name in self.log.drain(mark..) {
            env.remove(&name);
        }
    }

    fn check_range(&self, e: &Expr, lo: i64, hi: i64) -> R<()> {
        if hi.saturating_sub(lo) >= MAX_RANGE {
            return Err(self
                .err(e, format!("The range {lo}..{hi} is too large"))
                .explain(format!("Ranges may have at most {MAX_RANGE} elements here. A bound is probably wrong (or a test value is much larger than intended).")));
        }
        Ok(())
    }

    fn err(&self, e: &Expr, title: impl Into<String>) -> Diagnostic {
        let (l, c) = self.spec.pos(e.start);
        Diagnostic::new(l, c, title)
    }
    fn t(&self, e: &Expr) -> &str {
        self.spec.text(e)
    }
    pub fn is_unbound(&self, env: &Env, name: &str) -> bool {
        self.spec.decl(name).is_some() && !env.contains_key(name)
    }

    pub fn truth(&mut self, e: &Expr, env: &mut Env) -> R<bool> {
        match self.eval(e, env)? {
            Value::Bool(b) => Ok(b),
            v => Err(self
                .err(e, format!("`{}` should be true or false, but it is {}", self.t(e), v.kind()))
                .explain(format!("Its value is {}. Conditions (and, or, ->, quantifier bodies, COUNT/SEARCH conditions…) need a logical value.", v.short()))),
        }
    }

    fn int(&mut self, e: &Expr, env: &mut Env, what: &str) -> R<i64> {
        match self.eval(e, env)? {
            Value::Int(i) => Ok(i),
            Value::Real(r) if r.fract() == 0.0 => Ok(r as i64),
            Value::Undef => Err(self
                .err(e, format!("{what} `{}` has no defined value", self.t(e)))
                .explain("It comes from a pattern result that is meaningless here, e.g. the index of a SEARCH that found nothing.")
                .hint("Only use it when the search succeeded, e.g. `van -> …`.")),
            v => Err(self.err(e, format!("{what} `{}` must be a whole number, but it is {}", self.t(e), v.short()))),
        }
    }

    pub fn eval(&mut self, e: &Expr, env: &mut Env) -> R<Value> {
        self.steps += 1;
        if self.steps.is_multiple_of(4096)
            && self.cancel.is_some_and(|c| c.load(AtomicOrdering::Relaxed))
        {
            return Err(Diagnostic::plain("Evaluation cancelled"));
        }
        if self.steps > MAX_STEPS {
            return Err(self
                .err(e, "Evaluation takes too long")
                .explain("The specification needed more than 50 million steps on this test case. A range is probably much larger than intended."));
        }
        Ok(match &*e.kind {
            ExprKind::Int(i) => Value::Int(*i),
            ExprKind::Real(r) => Value::Real(*r),
            ExprKind::Str(s) => Value::Str(s.clone()),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Var(n) => match env.get(n) {
                Some(v) => v.clone(),
                None => {
                    return Err(self
                        .err(e, format!("`{n}` has no value at this point"))
                        .explain(format!(
                            "`{n}` is not in the test data, and nothing to the left of this point gave it a value. The editor only fills in a missing variable from an equation like `{n}=…` (or a pattern such as `({n},…)=SEARCH(…)`) that is evaluated before it is used, reading left to right."
                        ))
                        .hint(format!("Add `{n}: …` to the test case, or move the part that determines `{n}` earlier in the condition.")));
                }
            },
            ExprKind::Tuple(slots) => {
                let mut v = Vec::new();
                for s in slots {
                    match s {
                        Some(x) => v.push(self.eval(x, env)?),
                        None => return Err(self.err(
                            e,
                            "An empty slot like `(a,,b)` is only allowed on the left side of `=`",
                        )),
                    }
                }
                Value::Tuple(v, &[])
            }
            ExprKind::Array(items) => {
                let mut v = Vec::new();
                for x in items {
                    v.push(self.eval(x, env)?);
                }
                Value::Arr { from: 1, items: v }
            }
            ExprKind::Neg(a) => match self.eval(a, env)? {
                Value::Int(i) => Value::Int(
                    i.checked_neg()
                        .ok_or_else(|| self.err(e, "Number overflow"))?,
                ),
                Value::Real(r) => Value::Real(-r),
                Value::Undef => Value::Undef,
                v => {
                    return Err(self.err(
                        e,
                        format!("Can't negate `{}`: it is {}", self.t(a), v.kind()),
                    ));
                }
            },
            ExprKind::Not(a) => {
                let mark = self.log.len();
                let r = self.truth(a, env)?;
                self.rollback(mark, env);
                Value::Bool(!r)
            }
            ExprKind::Bin(op, a, b) => match op {
                BinOp::And => Value::Bool(self.truth(a, env)? && self.truth(b, env)?),
                BinOp::Or => {
                    let mark = self.log.len();
                    if self.truth(a, env)? {
                        Value::Bool(true)
                    } else {
                        self.rollback(mark, env);
                        Value::Bool(self.truth(b, env)?)
                    }
                }
                BinOp::Imp => {
                    let mark = self.log.len();
                    if self.truth(a, env)? {
                        Value::Bool(self.truth(b, env)?)
                    } else {
                        self.rollback(mark, env);
                        Value::Bool(true)
                    }
                }
                _ => {
                    let x = self.eval(a, env)?;
                    let y = self.eval(b, env)?;
                    self.arith(e, *op, a, b, x, y)?
                }
            },
            ExprKind::Cmp(ops_e, ops) => {
                if ops.len() == 1 && ops[0] == "=" {
                    return Ok(Value::Bool(self.assign(&ops_e[0], &ops_e[1], env)?));
                }
                let mut prev = self.eval(&ops_e[0], env)?;
                for (i, op) in ops.iter().enumerate() {
                    let next = self.eval(&ops_e[i + 1], env)?;
                    if !self.compare(e, op, &ops_e[i], &ops_e[i + 1], &prev, &next)? {
                        return Ok(Value::Bool(false));
                    }
                    prev = next;
                }
                Value::Bool(true)
            }
            ExprKind::In(x, lo, hi) => {
                let v = self.eval(x, env)?;
                let l = self.eval(lo, env)?;
                let h = self.eval(hi, env)?;
                if !v.is_num() || !l.is_num() || !h.is_num() {
                    return Err(self.err(
                        e,
                        format!("`{}`: membership in an interval needs numbers", self.t(e)),
                    ));
                }
                Value::Bool(l.num() <= v.num() && v.num() <= h.num())
            }
            ExprKind::Index(base, idx) => {
                let mut is = Vec::new();
                for ie in idx {
                    is.push(self.int(ie, env, "The index")?);
                }
                // borrow variables instead of cloning whole arrays on every access
                let owned;
                let mut v: &Value = match &*base.kind {
                    ExprKind::Var(n) if env.contains_key(n) => &env[n],
                    _ => {
                        owned = self.eval(base, env)?;
                        &owned
                    }
                };
                for (k, (ie, &i)) in idx.iter().zip(&is).enumerate() {
                    v = match v {
                        Value::Arr { from, items } => {
                            let from = *from;
                            let len = items.len() as i64;
                            if i < from || i >= from + len {
                                let name = if k == 0 {
                                    self.t(base).to_string()
                                } else {
                                    format!("row of `{}`", self.t(base))
                                };
                                let range = if len == 0 {
                                    "it is empty".to_string()
                                } else {
                                    format!("its indexes are {from}..{}", from + len - 1)
                                };
                                return Err(self
                                    .err(ie, format!("`{}`: index {i} is out of range", self.t(e)))
                                    .explain(format!("`{name}` has {len} element(s), {range}."))
                                    .hint("Check the loop bounds; when you look at `x[i+1]`, the loop must stop at n-1."));
                            }
                            &items[(i - from) as usize]
                        }
                        other => {
                            return Err(self
                                .err(e, format!("`{}` can't be indexed", self.t(base)))
                                .explain(format!("Only arrays (and matrices) can be indexed with `[…]`, but this is {}: {}.", other.kind(), other.short())));
                        }
                    };
                }
                v.clone()
            }
            ExprKind::Field(base, f) => {
                let v = self.eval(base, env)?;
                self.field(e, base, v, f)?
            }
            ExprKind::Call(name, args) => self.call(e, name, args, env)?,
            ExprKind::Quant {
                all,
                var,
                lo,
                hi,
                body,
            } => {
                let l = self.int(lo, env, "The lower bound")?;
                let h = self.int(hi, env, "The upper bound")?;
                self.check_range(e, l, h)?;
                let old = env.remove(var);
                let mut result = *all;
                let mut i = l;
                while i <= h {
                    env.insert(var.clone(), Value::Int(i));
                    let mark = self.log.len();
                    let ok = match self.truth(body, env) {
                        Ok(ok) => ok,
                        Err(d) => {
                            restore(env, var, old);
                            return Err(d);
                        }
                    };
                    // ∃ keeps the bindings of its witness; everything else is undone
                    if !all && ok {
                        result = true;
                        break;
                    }
                    self.rollback(mark, env);
                    if *all && !ok {
                        result = false;
                        break;
                    }
                    i += 1;
                }
                restore(env, var, old);
                Value::Bool(result)
            }
            ExprKind::Macro {
                name,
                var,
                lo,
                hi,
                args,
            } => {
                let old = env.remove(var);
                let r = self.pattern(e, name, var, lo, hi.as_ref(), args, env);
                restore(env, var, old);
                r?
            }
        })
    }

    fn arith(&self, e: &Expr, op: BinOp, a: &Expr, b: &Expr, x: Value, y: Value) -> R<Value> {
        use Value::*;
        let sym = match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            _ => "mod",
        };
        if matches!(x, Undef) || matches!(y, Undef) {
            return Ok(Undef);
        }
        if op == BinOp::Add
            && let (Str(s), Str(t)) = (&x, &y)
        {
            return Ok(Str(format!("{s}{t}")));
        }
        if !x.is_num() || !y.is_num() {
            let (bad, v) = if !x.is_num() { (a, &x) } else { (b, &y) };
            return Err(self
                .err(
                    e,
                    format!(
                        "`{sym}` needs numbers, but `{}` is {}",
                        self.t(bad),
                        v.kind()
                    ),
                )
                .explain(format!("Its value is {}.", v.short())));
        }
        let overflow = || {
            self.err(e, "Number overflow")
                .explain("The result does not fit in a 64-bit whole number.")
        };
        Ok(match (op, &x, &y) {
            (BinOp::Div | BinOp::Mod, _, _) if y.num() == 0.0 => {
                return Err(self
                    .err(e, format!("Division by zero in `{}`", self.t(e)))
                    .explain(format!("`{}` is 0 here.", self.t(b))));
            }
            (BinOp::Add, Int(p), Int(q)) => Int(p.checked_add(*q).ok_or_else(overflow)?),
            (BinOp::Sub, Int(p), Int(q)) => Int(p.checked_sub(*q).ok_or_else(overflow)?),
            (BinOp::Mul, Int(p), Int(q)) => Int(p.checked_mul(*q).ok_or_else(overflow)?),
            (BinOp::Mod, Int(p), Int(q)) => Int(p.checked_rem_euclid(*q).ok_or_else(overflow)?),
            (BinOp::Div, Int(p), Int(q)) if p.checked_rem(*q) == Some(0) => {
                Int(p.checked_div(*q).ok_or_else(overflow)?)
            }
            (BinOp::Add, ..) => Real(x.num() + y.num()),
            (BinOp::Sub, ..) => Real(x.num() - y.num()),
            (BinOp::Mul, ..) => Real(x.num() * y.num()),
            (BinOp::Div, ..) => Real(x.num() / y.num()),
            (BinOp::Mod, ..) => Real(x.num().rem_euclid(y.num())),
            _ => unreachable!(),
        })
    }

    fn compare(&self, e: &Expr, op: &str, a: &Expr, b: &Expr, x: &Value, y: &Value) -> R<bool> {
        match op {
            "=" => return Ok(x.eq_val(y)),
            "!=" => return Ok(!x.eq_val(y)),
            "|" => {
                return match (x, y) {
                    (Value::Int(p), Value::Int(q)) => Ok(if *p == 0 {
                        *q == 0
                    } else {
                        q.checked_rem(*p).is_none_or(|r| r == 0)
                    }),
                    _ => Err(self
                        .err(
                            e,
                            format!("`{}`: `|` (divides) needs whole numbers", self.t(e)),
                        )
                        .explain(format!(
                            "`{}` is {} and `{}` is {}.",
                            self.t(a),
                            x.short(),
                            self.t(b),
                            y.short()
                        ))),
                };
            }
            _ => {}
        }
        let ord = match (x, y) {
            (Value::Undef, _) | (_, Value::Undef) => {
                let bad = if matches!(x, Value::Undef) { a } else { b };
                return Err(self
                    .err(bad, format!("`{}` has no defined value", self.t(bad)))
                    .explain("It is a pattern result with no meaning here, e.g. the index of a SEARCH that found nothing.")
                    .hint("Guard it with an implication like `van -> …`."));
            }
            (Value::Str(p), Value::Str(q)) => p.cmp(q),
            _ if x.is_num() && y.is_num() => {
                x.num().partial_cmp(&y.num()).unwrap_or(Ordering::Equal)
            }
            _ => {
                return Err(self
                    .err(
                        e,
                        format!(
                            "Can't compare `{}` with `{}` using `{op}`",
                            self.t(a),
                            self.t(b)
                        ),
                    )
                    .explain(format!(
                        "The left side is {} ({}) and the right side is {} ({}).",
                        x.kind(),
                        x.short(),
                        y.kind(),
                        y.short()
                    )));
            }
        };
        Ok(match op {
            "<" => ord == Ordering::Less,
            ">" => ord == Ordering::Greater,
            "<=" => ord != Ordering::Greater,
            _ => ord != Ordering::Less,
        })
    }

    /// Does this `=` side contain a variable that has no value yet (so `=` assigns)?
    fn has_hole(&self, e: &Expr, env: &Env) -> bool {
        match &*e.kind {
            ExprKind::Var(n) => self.is_unbound(env, n),
            ExprKind::Neg(x) => matches!(&*x.kind, ExprKind::Var(n) if self.is_unbound(env, n)),
            ExprKind::Tuple(s) => s
                .iter()
                .any(|x| x.as_ref().is_none_or(|x| self.has_hole(x, env))),
            _ => false,
        }
    }

    fn assign(&mut self, l: &Expr, r: &Expr, env: &mut Env) -> R<bool> {
        if self.has_hole(l, env) {
            let v = self.eval(r, env)?;
            return self.bind(l, v, env, r);
        }
        if self.has_hole(r, env) {
            let v = self.eval(l, env)?;
            return self.bind(r, v, env, l);
        }
        let a = self.eval(l, env)?;
        let b = self.eval(r, env)?;
        Ok(a.eq_val(&b))
    }

    fn bind(&mut self, target: &Expr, v: Value, env: &mut Env, source: &Expr) -> R<bool> {
        match &*target.kind {
            ExprKind::Var(n) if self.is_unbound(env, n) => {
                self.log.push(n.clone());
                env.insert(n.clone(), v);
                Ok(true)
            }
            ExprKind::Neg(x) if matches!(&*x.kind, ExprKind::Var(n) if self.is_unbound(env, n)) => {
                let ExprKind::Var(n) = &*x.kind else {
                    unreachable!()
                };
                let neg = match v {
                    Value::Int(i) => Value::Int(-i),
                    Value::Real(r) => Value::Real(-r),
                    other => other,
                };
                self.log.push(n.clone());
                env.insert(n.clone(), neg);
                Ok(true)
            }
            ExprKind::Tuple(slots) => {
                let Value::Tuple(items, _) = &v else {
                    return Err(self
                        .err(
                            target,
                            format!(
                                "`{}` has {} slots, but `{}` is not a tuple",
                                self.t(target),
                                slots.len(),
                                self.t(source)
                            ),
                        )
                        .explain(format!("The right side is {} ({}).", v.kind(), v.short())));
                };
                if items.len() != slots.len() {
                    return Err(self
                        .err(target, format!("`{}` has {} slots, but `{}` gives {} values", self.t(target), slots.len(), self.t(source), items.len()))
                        .explain("MAX/MIN give (index, value); SEARCH gives (found, index); MIS gives (count, array); CONDMAX gives (found, index, value).")
                        .hint("Leave a slot empty to skip it, like `(,value)=MAX(…)` or `(van,,value)=CONDMAX(…)`."));
                }
                let mut ok = true;
                for (s, item) in slots.iter().zip(items.clone()) {
                    if let Some(s) = s {
                        ok &= self.bind(s, item, env, source)?;
                    }
                }
                Ok(ok)
            }
            _ => {
                let t = self.eval(target, env)?;
                Ok(t.eq_val(&v))
            }
        }
    }

    fn field(&self, e: &Expr, base: &Expr, v: Value, f: &str) -> R<Value> {
        match v {
            Value::Rec(fs) => {
                let names: Vec<String> = fs.iter().map(|(k, _)| k.clone()).collect();
                fs.into_iter()
                    .find(|(k, _)| k == f)
                    .map(|(_, v)| v)
                    .ok_or_else(|| {
                        self.err(e, format!("`{}` has no field `{f}`", self.t(base)))
                            .explain(format!(
                                "Its fields are: {}. Field names are case-sensitive.",
                                names.join(", ")
                            ))
                    })
            }
            Value::Tuple(items, names) => {
                let idx = f
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| i.checked_sub(1))
                    .or_else(|| names.iter().position(|n| *n == f));
                match idx.and_then(|i| items.get(i)) {
                    Some(v) => Ok(v.clone()),
                    None => {
                        let mut opts: Vec<String> =
                            (1..=items.len()).map(|i| format!(".{i}")).collect();
                        opts.extend(names.iter().map(|n| format!(".{n}")));
                        Err(self
                            .err(
                                e,
                                format!("The tuple `{}` has no part `.{f}`", self.t(base)),
                            )
                            .explain(format!(
                                "It has {} parts; use {}.",
                                items.len(),
                                opts.join(", ")
                            )))
                    }
                }
            }
            other => Err(self
                .err(e, format!("`{}` has no fields", self.t(base)))
                .explain(format!(
                    "`.{f}` works on records and tuples, but this is {}: {}.",
                    other.kind(),
                    other.short()
                ))),
        }
    }

    fn call(&mut self, e: &Expr, name: &str, args: &[Expr], env: &mut Env) -> R<Value> {
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.eval(a, env)?);
        }
        let need_arr = |s: &Self, v: &Value| -> R<(i64, i64)> {
            match v {
                Value::Arr { from, items } => Ok((*from, items.len() as i64)),
                Value::Str(t) => Ok((1, t.chars().count() as i64)),
                other => Err(s.err(
                    e,
                    format!(
                        "`{name}` needs an array, but `{}` is {}",
                        s.t(&args[0]),
                        other.kind()
                    ),
                )),
            }
        };
        match name {
            "length" => return Ok(Value::Int(need_arr(self, &vals[0])?.1)),
            "from" => return Ok(Value::Int(need_arr(self, &vals[0])?.0)),
            "to" => {
                let (f, l) = need_arr(self, &vals[0])?;
                return Ok(Value::Int(f + l - 1));
            }
            "ABS" | "abs" => {
                return match &vals[0] {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Real(r) => Ok(Value::Real(r.abs())),
                    v => Err(self.err(
                        e,
                        format!(
                            "ABS needs a number, but `{}` is {}",
                            self.t(&args[0]),
                            v.kind()
                        ),
                    )),
                };
            }
            _ => {}
        }
        let f = self.spec.funcs.get(name).expect("checked by the parser");
        if self.depth > 500 {
            return Err(self
                .err(e, format!("`{name}` calls itself too deeply"))
                .explain(
                    "More than 500 nested function calls; the recursion probably never stops.",
                ));
        }
        let saved: Vec<(String, Option<Value>)> = f
            .params
            .iter()
            .zip(vals)
            .map(|(p, v)| (p.clone(), env.insert(p.clone(), v)))
            .collect();
        let mark = self.log.len();
        self.depth += 1;
        let r = self.eval(&f.body, env);
        self.depth -= 1;
        self.rollback(mark, env);
        for (p, old) in saved.into_iter().rev() {
            restore(env, &p, old);
        }
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn pattern(
        &mut self,
        e: &Expr,
        name: &str,
        var: &str,
        lo: &Expr,
        hi: Option<&Expr>,
        args: &[Expr],
        env: &mut Env,
    ) -> R<Value> {
        let b = self.int(lo, env, "The range start")?;
        if name == "SELECT" {
            for i in b..b.saturating_add(1_000_000) {
                env.insert(var.to_string(), Value::Int(i));
                if self.truth(&args[0], env)? {
                    return Ok(Value::Int(i));
                }
            }
            return Err(self
                .err(e, "SELECT found no matching element")
                .explain(format!("No `{var}` ≥ {b} satisfies `{}` (searched a million values).", self.t(&args[0])))
                .hint("SELECT assumes a match exists. Prove it in the precondition (e.g. `∃i∈[1..n]:(…)`), or use SEARCH if it may not exist."));
        }
        let end = self.int(hi.unwrap(), env, "The range end")?;
        self.check_range(e, b, end)?;
        let set = |env: &mut Env, i: i64| {
            env.insert(var.to_string(), Value::Int(i));
        };
        let range = b..=end;
        let add = |s: &Self, acc: Value, v: Value, at: &Expr| -> R<Value> {
            match (&acc, &v) {
                (Value::Int(p), Value::Int(q)) => Ok(Value::Int(
                    p.checked_add(*q)
                        .ok_or_else(|| s.err(e, "Number overflow in SUM"))?,
                )),
                _ if v.is_num() => Ok(Value::Real(acc.num() + v.num())),
                _ => Err(s
                    .err(
                        at,
                        format!("SUM needs numbers, but `{}` is {}", s.t(at), v.kind()),
                    )
                    .explain(format!("Its value is {}.", v.short()))),
            }
        };
        match name {
            "SUM" => {
                let mut acc = Value::Int(0);
                for i in range {
                    set(env, i);
                    if args.len() == 2 && !self.truth(&args[1], env)? {
                        continue;
                    }
                    let v = self.eval(&args[0], env)?;
                    acc = add(self, acc, v, &args[0])?;
                }
                Ok(acc)
            }
            "COUNT" | "DARAB" => {
                let mut n = 0;
                for i in range {
                    set(env, i);
                    n += self.truth(&args[0], env)? as i64;
                }
                Ok(Value::Int(n))
            }
            "EXISTS" | "FORALL" => {
                let all = name == "FORALL";
                for i in range {
                    set(env, i);
                    if self.truth(&args[0], env)? != all {
                        return Ok(Value::Bool(!all));
                    }
                }
                Ok(Value::Bool(all))
            }
            "SEARCH" => {
                for i in range {
                    set(env, i);
                    if self.truth(&args[0], env)? {
                        return Ok(Value::Tuple(
                            vec![Value::Bool(true), Value::Int(i)],
                            &["found", "ind"],
                        ));
                    }
                }
                Ok(Value::Tuple(
                    vec![Value::Bool(false), Value::Undef],
                    &["found", "ind"],
                ))
            }
            "COPY" => {
                let mut items = Vec::new();
                for i in range {
                    set(env, i);
                    items.push(self.eval(&args[0], env)?);
                }
                Ok(Value::Arr { from: b, items })
            }
            "MIS" => {
                let mut items = Vec::new();
                for i in range {
                    set(env, i);
                    if self.truth(&args[0], env)? {
                        items.push(if args.len() == 2 {
                            self.eval(&args[1], env)?
                        } else {
                            Value::Int(i)
                        });
                    }
                }
                Ok(Value::Tuple(
                    vec![
                        Value::Int(items.len() as i64),
                        Value::Arr { from: 1, items },
                    ],
                    &["cnt", "items"],
                ))
            }
            "MAX" | "MIN" | "CONDMAX" | "CONDMIN" => {
                let want = if name.ends_with("MAX") {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
                let cond = name.starts_with("COND");
                let mut best: Option<(i64, Value)> = None;
                for i in range {
                    set(env, i);
                    if cond && !self.truth(&args[1], env)? {
                        continue;
                    }
                    let v = self.eval(&args[0], env)?;
                    let better = match &best {
                        None => true,
                        Some((_, bv)) => {
                            let ord = match (&v, bv) {
                                (Value::Str(p), Value::Str(q)) => p.cmp(q),
                                _ if v.is_num() && bv.is_num() => {
                                    v.num().partial_cmp(&bv.num()).unwrap_or(Ordering::Equal)
                                }
                                _ => {
                                    return Err(self
                                        .err(&args[0], format!("{name} can only compare numbers or texts, but `{}` is {}", self.t(&args[0]), v.kind())));
                                }
                            };
                            ord == want
                        }
                    };
                    if better {
                        best = Some((i, v));
                    }
                }
                let names: &'static [&'static str] = match name {
                    "MAX" => &["maxind", "maxval"],
                    "MIN" => &["minind", "minval"],
                    "CONDMAX" => &["found", "maxind", "maxval"],
                    _ => &["found", "minind", "minval"],
                };
                match (best, cond) {
                    (Some((i, v)), false) => Ok(Value::Tuple(vec![Value::Int(i), v], names)),
                    (Some((i, v)), true) => Ok(Value::Tuple(vec![Value::Bool(true), Value::Int(i), v], names)),
                    (None, true) => Ok(Value::Tuple(vec![Value::Bool(false), Value::Undef, Value::Undef], names)),
                    (None, false) => Err(self
                        .err(e, format!("{name} over an empty range ({b}..{end})"))
                        .explain(format!("{name} needs at least one element, but the range `{}..{}` is empty for this test case.", self.t(lo), self.t(hi.unwrap())))
                        .hint(format!("Require a non-empty range in the precondition (e.g. `n>0`), or use COND{name} if the range may be empty."))),
                }
            }
            _ => unreachable!("unknown macro {name}"),
        }
    }
}

/// Put back the value a loop variable or parameter shadowed.
fn restore(env: &mut Env, name: &str, old: Option<Value>) {
    match old {
        Some(v) => {
            env.insert(name.to_string(), v);
        }
        None => {
            env.remove(name);
        }
    }
}
