//! The expression / theorem-macro evaluator.

mod bind;
mod call;
mod ops;
mod pattern;
mod trace;
mod value;

pub use pattern::result_names;
pub use trace::{Step, StepKind, TRACE_CAP, Trace};
pub use value::{Env, Value, set_from};

use crate::diag::Diagnostic;
use crate::parser::{BinOp, Expr, ExprKind, Spec};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use value::{restore, set};

type R<T> = Result<T, Diagnostic>;

pub type Calls<'a> = dyn Fn(&str, &[Value], &Env) -> Option<Result<Value, Diagnostic>> + 'a;

const MAX_STEPS: u64 = 50_000_000;
/// Longest loop a quantifier or theorem macro may run (also bounds memory for COPY/MIS).
const MAX_RANGE: i64 = 10_000_000;
const MAX_NESTING: u32 = 4000;
pub const EVAL_STACK: usize = 64 << 20;

pub struct Eval<'a> {
    pub spec: &'a Spec,
    depth: usize,
    nesting: u32,
    steps: u64,
    /// Variables bound by `=` so far, so failed branches can be undone without cloning the environment.
    log: Vec<String>,
    cancel: Option<&'a AtomicBool>,
    /// When set, every evaluated sub-expression is recorded (up to [`TRACE_CAP`] steps).
    pub trace: Option<Trace>,
    root: (usize, usize),
    root_label: &'a str,
    scope_names: Vec<String>,
    error_recorded: bool,
    trace_full: bool,
    pub calls: Option<&'a Calls<'a>>,
    pub compare_only: bool,
}

impl<'a> Eval<'a> {
    pub fn new(spec: &'a Spec) -> Self {
        Eval {
            spec,
            depth: 0,
            nesting: 0,
            steps: 0,
            log: Vec::new(),
            cancel: None,
            trace: None,
            root: (0, 0),
            root_label: "",
            scope_names: Vec::new(),
            error_recorded: false,
            trace_full: false,
            calls: None,
            compare_only: false,
        }
    }

    /// Evaluation stops with an error as soon as `cancel` becomes true.
    pub fn with_cancel(spec: &'a Spec, cancel: &'a AtomicBool) -> Self {
        Eval {
            cancel: Some(cancel),
            ..Self::new(spec)
        }
    }

    pub fn eval_scratch(&mut self, e: &Expr, env: &mut Env) -> R<Value> {
        let mark = self.log.len();
        let r = self.eval(e, env);
        self.rollback(mark, env);
        r
    }

    pub fn truth_scratch(&mut self, e: &Expr, env: &mut Env) -> R<bool> {
        let mark = self.log.len();
        let r = self.truth(e, env);
        self.rollback(mark, env);
        r
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
                .doc("ch08-00-evaluation.html").explain(format!("Ranges may have at most {MAX_RANGE} elements here. A bound is probably wrong (or a test value is much larger than intended).")));
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
        !self.compare_only && self.spec.decl(name).is_some() && !env.contains_key(name)
    }

    pub fn truth(&mut self, e: &Expr, env: &mut Env) -> R<bool> {
        match self.eval(e, env)? {
            Value::Bool(b) => Ok(b),
            v => Err(self
                .err(e, format!("`{}` should be true or false, but it is {}", self.t(e), v.kind()))
                .doc("ch04-01-operators.html").explain(format!("Its value is {}. Conditions (and, or, ->, quantifier bodies, COUNT/SEARCH conditions…) need a logical value.", v.short()))),
        }
    }

    fn int(&mut self, e: &Expr, env: &mut Env, what: &str) -> R<i64> {
        match self.eval(e, env)? {
            Value::Int(i) => Ok(i),
            Value::Real(r) if r.fract() == 0.0 => Ok(r as i64),
            Value::Undef => Err(self
                .err(e, format!("{what} `{}` has no defined value", self.t(e)))
                .doc("ch06-06-search.html").explain("It comes from a pattern result that is meaningless here, e.g. the index of a SEARCH that found nothing.")
                .hint("Only use it when the search succeeded, e.g. `van -> …`.")),
            v => Err(self.err(e, format!("{what} `{}` must be a whole number, but it is {}", self.t(e), v.short()))),
        }
    }

    /// Set the formula that following steps belong to (for the stepper).
    pub fn set_root(&mut self, e: &Expr, label: &'a str) {
        self.root = (e.start, e.end);
        self.root_label = label;
    }

    pub(super) fn push_scope(&mut self, name: &str) {
        if self.trace.is_some() {
            self.scope_names.push(name.to_string());
        }
    }

    pub(super) fn pop_scope(&mut self, n: usize) {
        let keep = self.scope_names.len().saturating_sub(n);
        self.scope_names.truncate(keep);
    }

    fn record(&mut self, e: &Expr, kind: StepKind, env: &Env) {
        let Some(t) = &mut self.trace else { return };
        if t.steps.len() >= TRACE_CAP {
            t.truncated = true;
            self.trace_full = true;
            return;
        }
        let mut scope: Vec<(String, String)> = Vec::new();
        for n in self.scope_names.iter().rev() {
            if !scope.iter().any(|(m, _)| m == n)
                && let Some(v) = env.get(n)
            {
                scope.push((n.clone(), v.short()));
            }
        }
        scope.reverse();
        t.steps.push(Step {
            root: self.root,
            label: self.root_label.to_string(),
            span: (e.start, e.end),
            kind,
            depth: self.depth,
            scope,
        });
    }

    pub fn eval(&mut self, e: &Expr, env: &mut Env) -> R<Value> {
        if self.nesting >= MAX_NESTING {
            return Err(self.too_deep(e));
        }
        self.nesting += 1;
        let r = self.eval_inner(e, env);
        self.nesting -= 1;
        if self.trace.is_some() {
            self.record_result(e, &r, env);
        }
        r
    }

    #[cold]
    fn too_deep(&self, e: &Expr) -> Diagnostic {
        self.err(e, "The evaluation is nested too deeply")
            .doc("ch05-00-functions.html").explain(format!(
                "More than {MAX_NESTING} sub-expressions are being evaluated inside each other. A function probably calls itself without ever stopping."
            ))
    }

    #[cold]
    fn record_result(&mut self, e: &Expr, r: &R<Value>, env: &Env) {
        let literal = matches!(
            &*e.kind,
            ExprKind::Int(_) | ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Bool(_)
        );
        if literal {
            return;
        }
        match r {
            Ok(v) => self.record(e, StepKind::Value(v.short()), env),
            Err(d) if !self.error_recorded => {
                self.error_recorded = true;
                self.record(e, StepKind::Error(d.title.clone()), env);
            }
            Err(_) => {}
        }
    }

    fn count_step(&mut self, e: &Expr) -> R<()> {
        if self.trace_full {
            return Err(Diagnostic::plain("The stepper has all the steps it shows"));
        }
        self.steps += 1;
        if self.steps.is_multiple_of(4096)
            && self.cancel.is_some_and(|c| c.load(AtomicOrdering::Relaxed))
        {
            return Err(Diagnostic::plain("Evaluation cancelled"));
        }
        if self.steps > MAX_STEPS {
            return Err(self
                .err(e, "Evaluation takes too long")
                .doc("ch08-00-evaluation.html").explain("The specification needed more than 50 million steps on this test case. A range is probably much larger than intended."));
        }
        Ok(())
    }

    fn eval_inner(&mut self, e: &Expr, env: &mut Env) -> R<Value> {
        self.count_step(e)?;
        match &*e.kind {
            ExprKind::Int(i) => Ok(Value::Int(*i)),
            ExprKind::Real(r) => Ok(Value::Real(*r)),
            ExprKind::Str(s) => Ok(Value::Str(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Var(n) => match env.get(n) {
                Some(v) => Ok(v.clone()),
                None => Err(self.no_value(e, n)),
            },
            ExprKind::Tuple(slots) => self.tuple(e, slots, env),
            ExprKind::Array(items, from) => self.array(items, from, env),
            ExprKind::Neg(a) => self.negate(e, a, env),
            ExprKind::Not(a) => Ok(Value::Bool(!self.truth_scratch(a, env)?)),
            ExprKind::Bin(op, a, b) => self.binary(e, *op, a, b, env),
            ExprKind::Cmp(operands, ops) => self.comparison(e, operands, ops, env),
            ExprKind::In(x, lo, hi) => self.member(e, x, lo, hi, env),
            ExprKind::Index(base, idx) => self.index(e, base, idx, env),
            ExprKind::Field(base, f) => {
                let v = self.eval(base, env)?;
                self.field(e, base, v, f)
            }
            ExprKind::Call(name, args) => self.call(e, name, args, env),
            ExprKind::Quant {
                all,
                var,
                lo,
                hi,
                body,
            } => Ok(Value::Bool(self.quant(e, *all, var, lo, hi, body, env)?)),
            ExprKind::Macro {
                name,
                var,
                lo,
                hi,
                args,
            } => {
                let old = env.remove(var);
                self.push_scope(var);
                let r = self.pattern(e, name, var, lo, hi.as_ref(), args, env);
                self.pop_scope(1);
                restore(env, var, old);
                r
            }
            ExprKind::Cases(arms, otherwise) => {
                for (value, cond) in arms {
                    // like `nem`: an equation in a condition gives nothing a value
                    if self.truth_scratch(cond, env)? {
                        return self.eval(value, env);
                    }
                }
                match otherwise {
                    Some(v) => self.eval(v, env),
                    None => Err(self
                        .err(e, format!("None of the cases of `{}` holds", self.t(e)))
                        .hint("Add a last case for the rest: `…; x egyébként}`.")),
                }
            }
        }
    }

    #[cold]
    fn no_value(&self, e: &Expr, n: &str) -> Diagnostic {
        self.err(e, format!("`{n}` has no value at this point"))
            .explain(format!(
                "`{n}` is not in the test data, and nothing to the left of this point gave it a value. The editor only fills in a missing variable from an equation like `{n}=…` (or a pattern such as `({n},…)=SEARCH(…)`) that is evaluated before it is used, reading left to right."
            ))
            .hint(format!("Add `{n}: …` to the test case, or move the part that determines `{n}` earlier in the condition."))
    }

    fn tuple(&mut self, e: &Expr, slots: &[Option<Expr>], env: &mut Env) -> R<Value> {
        let mut v = Vec::with_capacity(slots.len());
        for s in slots {
            match s {
                Some(x) => v.push(self.eval(x, env)?),
                None => {
                    return Err(self.err(
                        e,
                        "An empty slot like `(a,,b)` is only allowed on the left side of `=`",
                    ));
                }
            }
        }
        Ok(Value::Tuple(v, &[]))
    }

    fn array(&mut self, items: &[Expr], from: &[Expr], env: &mut Env) -> R<Value> {
        let mut v = Vec::with_capacity(items.len());
        for x in items {
            v.push(self.eval(x, env)?);
        }
        let mut arr = Value::Arr {
            from: 1,
            items: v.into(),
        };
        let mut starts = Vec::with_capacity(from.len());
        for f in from {
            starts.push(self.int(f, env, "The start index")?);
        }
        set_from(&mut arr, &starts);
        Ok(arr)
    }

    fn negate(&mut self, e: &Expr, a: &Expr, env: &mut Env) -> R<Value> {
        match self.eval(a, env)? {
            Value::Int(i) => i
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| self.err(e, "Number overflow")),
            Value::Real(r) => Ok(Value::Real(-r)),
            Value::Undef => Ok(Value::Undef),
            v => Err(self.err(
                e,
                format!("Can't negate `{}`: it is {}", self.t(a), v.kind()),
            )),
        }
    }

    fn binary(&mut self, e: &Expr, op: BinOp, a: &Expr, b: &Expr, env: &mut Env) -> R<Value> {
        let truth = match op {
            BinOp::And => self.truth(a, env)? && self.truth(b, env)?,
            BinOp::Or => {
                let mark = self.log.len();
                if self.truth(a, env)? {
                    true
                } else {
                    self.rollback(mark, env);
                    self.truth(b, env)?
                }
            }
            BinOp::Imp => {
                let mark = self.log.len();
                if self.truth(a, env)? {
                    self.truth(b, env)?
                } else {
                    self.rollback(mark, env);
                    true
                }
            }
            _ => {
                let x = self.eval(a, env)?;
                let y = self.eval(b, env)?;
                return self.arith(e, op, a, b, x, y);
            }
        };
        Ok(Value::Bool(truth))
    }

    fn comparison(
        &mut self,
        e: &Expr,
        operands: &[Expr],
        ops: &[&'static str],
        env: &mut Env,
    ) -> R<Value> {
        if let (["="], [l, r]) = (ops, operands) {
            return Ok(Value::Bool(self.assign(l, r, env)?));
        }
        let mut prev = self.eval(&operands[0], env)?;
        for (i, op) in ops.iter().enumerate() {
            let next = self.eval(&operands[i + 1], env)?;
            if !self.compare(e, op, &operands[i], &operands[i + 1], &prev, &next)? {
                return Ok(Value::Bool(false));
            }
            prev = next;
        }
        Ok(Value::Bool(true))
    }

    /// `x ∈ [lo..hi]`.
    fn member(&mut self, e: &Expr, x: &Expr, lo: &Expr, hi: &Expr, env: &mut Env) -> R<Value> {
        let v = self.eval(x, env)?;
        let l = self.eval(lo, env)?;
        let h = self.eval(hi, env)?;
        if !v.is_num() || !l.is_num() || !h.is_num() {
            return Err(self.err(
                e,
                format!("`{}`: membership in an interval needs numbers", self.t(e)),
            ));
        }
        Ok(Value::Bool(l.num() <= v.num() && v.num() <= h.num()))
    }

    fn index(&mut self, e: &Expr, base: &Expr, idx: &[Expr], env: &mut Env) -> R<Value> {
        let mut is = Vec::with_capacity(idx.len());
        for ie in idx {
            is.push(self.int(ie, env, "The index")?);
        }
        // borrow variables instead of cloning whole arrays on every access
        let owned;
        let variable = match &*base.kind {
            ExprKind::Var(n) => env.get(n),
            _ => None,
        };
        let mut v: &Value = match variable {
            Some(v) => v,
            None => {
                owned = self.eval(base, env)?;
                &owned
            }
        };
        for (k, (ie, &i)) in idx.iter().zip(&is).enumerate() {
            let Value::Arr { from, items } = v else {
                return Err(self
                    .err(e, format!("`{}` can't be indexed", self.t(base)))
                    .doc("ch03-02-arrays.html#indexing")
                    .explain(format!(
                        "Only arrays (and matrices) can be indexed with `[…]`, but this is {}: {}.",
                        v.kind(),
                        v.short()
                    )));
            };
            let found = i
                .checked_sub(*from)
                .and_then(|at| usize::try_from(at).ok())
                .and_then(|at| items.get(at));
            let Some(item) = found else {
                let name = if k == 0 {
                    self.t(base).to_string()
                } else {
                    format!("row of `{}`", self.t(base))
                };
                let len = items.len();
                let range = if len == 0 {
                    "it is empty".to_string()
                } else {
                    format!("its indexes are {from}..{}", last_index(*from, len))
                };
                return Err(self
                    .err(ie, format!("`{}`: index {i} is out of range", self.t(e)))
                    .doc("ch03-02-arrays.html#indexing").explain(format!("`{name}` has {len} element(s), {range}."))
                    .hint("Check the loop bounds; when you look at `x[i+1]`, the loop must stop at n-1."));
            };
            v = item;
        }
        Ok(v.clone())
    }

    #[allow(clippy::too_many_arguments)]
    fn quant(
        &mut self,
        e: &Expr,
        all: bool,
        var: &str,
        lo: &Expr,
        hi: &Expr,
        body: &Expr,
        env: &mut Env,
    ) -> R<bool> {
        let l = self.int(lo, env, "The lower bound")?;
        let h = self.int(hi, env, "The upper bound")?;
        self.check_range(e, l, h)?;
        let old = env.remove(var);
        self.push_scope(var);
        let r = self.quant_loop(all, var, l..=h, body, env);
        self.pop_scope(1);
        restore(env, var, old);
        r
    }

    fn quant_loop(
        &mut self,
        all: bool,
        var: &str,
        range: std::ops::RangeInclusive<i64>,
        body: &Expr,
        env: &mut Env,
    ) -> R<bool> {
        for i in range {
            set(env, var, Value::Int(i));
            let mark = self.log.len();
            let ok = self.truth(body, env)?;
            // ∃ keeps the bindings of its witness; everything else is undone
            if !all && ok {
                return Ok(true);
            }
            self.rollback(mark, env);
            if all && !ok {
                return Ok(false);
            }
        }
        Ok(all)
    }
}

fn last_index(from: i64, len: usize) -> i64 {
    from.saturating_add(len as i64).saturating_sub(1)
}
