//! Field access, built-in functions and calls of the specification's own functions.

use super::value::set;
use super::{Env, Eval, R, StepKind, Value, restore};
use crate::parser::{Expr, ExprKind};

impl Eval<'_> {
    pub(super) fn field(&self, e: &Expr, base: &Expr, v: Value, f: &str) -> R<Value> {
        match v {
            Value::Rec(fs) => fs
                .iter()
                .find(|(k, _)| k == f)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| {
                    let names: Vec<&str> = fs.iter().map(|(k, _)| k.as_str()).collect();
                    self.err(e, format!("`{}` has no field `{f}`", self.t(base)))
                        .explain(format!(
                            "Its fields are: {}. Field names are case-sensitive.",
                            names.join(", ")
                        ))
                }),
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

    pub(super) fn call(&mut self, e: &Expr, name: &str, args: &[Expr], env: &mut Env) -> R<Value> {
        match name {
            "length" | "hossz" | "from" | "tól" | "to" | "ig" => {
                let (from, len) = self.shape(e, name, &args[0], env)?;
                Ok(Value::Int(match name {
                    "length" | "hossz" => len,
                    "from" | "tól" => from,
                    _ => from.saturating_add(len).saturating_sub(1),
                }))
            }
            "abs" | "sqrt" | "round" | "kerekít" | "!" => {
                let v = self.eval(&args[0], env)?;
                self.numeric(e, name, &args[0], &v)
            }
            _ => self.call_defined(e, name, args, env),
        }
    }

    fn shape(&mut self, e: &Expr, name: &str, arg: &Expr, env: &mut Env) -> R<(i64, i64)> {
        let of = |s: &Self, v: &Value| match v {
            Value::Arr { from, items } => Ok((*from, items.len() as i64)),
            Value::Str(t) => Ok((1, t.chars().count() as i64)),
            other => Err(s.err(
                e,
                format!(
                    "`{name}` needs an array, but `{}` is {}",
                    s.t(arg),
                    other.kind()
                ),
            )),
        };
        if self.trace.is_none()
            && let ExprKind::Var(n) = &*arg.kind
            && let Some(v) = env.get(n)
        {
            return of(self, v);
        }
        let v = self.eval(arg, env)?;
        of(self, &v)
    }

    fn numeric(&self, e: &Expr, name: &str, arg: &Expr, v: &Value) -> R<Value> {
        if !v.is_num() {
            let what = if name == "!" { "`!` (factorial)" } else { name };
            return Err(self.err(
                e,
                format!(
                    "{what} needs a number, but `{}` is {}",
                    self.t(arg),
                    v.kind()
                ),
            ));
        }
        let overflow = || self.err(e, "Number overflow");
        Ok(match (name, v) {
            ("abs", Value::Int(i)) => Value::Int(i.checked_abs().ok_or_else(overflow)?),
            ("abs", _) => Value::Real(v.num().abs()),
            ("sqrt", _) => Value::Real(v.num().sqrt()),
            ("!", _) => {
                let n = v.num();
                if n < 0.0 || n.fract() != 0.0 {
                    return Err(self.err(
                        e,
                        format!("`{}`: the factorial needs a whole number ≥ 0", self.t(e)),
                    ));
                }
                let mut acc: i64 = 1;
                for k in 2..=(n as i64) {
                    acc = acc.checked_mul(k).ok_or_else(overflow)?;
                }
                Value::Int(acc)
            }
            (_, Value::Int(i)) => Value::Int(*i),
            // JavaScript Math.round: halves go up, round(-2.5) = -2
            _ => Value::Int((v.num() + 0.5).floor() as i64),
        })
    }

    fn call_defined(&mut self, e: &Expr, name: &str, args: &[Expr], env: &mut Env) -> R<Value> {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            vals.push(self.eval(a, env)?);
        }
        if let Some(answer) = self.calls.and_then(|calls| calls(name, &vals, env)) {
            return answer;
        }
        let spec = self.spec;
        let (name, f) = spec
            .funcs
            .get_key_value(name)
            .expect("checked by the parser");
        if self.depth > MAX_DEPTH {
            return Err(self
                .err(e, format!("`{name}` calls itself too deeply"))
                .explain(format!(
                    "More than {MAX_DEPTH} nested function calls; the recursion probably never stops."
                )));
        }
        if self.trace.is_some() {
            let shown: Vec<String> = vals.iter().map(Value::short).collect();
            self.record(
                e,
                StepKind::Enter(format!("{name}({})", shown.join(", "))),
                env,
            );
        }
        let shadowed: Vec<Option<Value>> = f
            .params
            .iter()
            .zip(vals)
            .map(|(p, v)| set(env, p, v))
            .collect();
        let mark = self.log.len();
        let outer = (self.root, self.root_label);
        self.set_root(&f.body, name);
        for p in &f.params {
            self.push_scope(p);
        }
        self.depth += 1;
        let r = self.eval(&f.body, env);
        self.depth -= 1;
        self.pop_scope(f.params.len());
        (self.root, self.root_label) = outer;
        if self.trace.is_some()
            && let Ok(v) = &r
        {
            self.record(e, StepKind::Leave(format!("{name} → {}", v.short())), env);
        }
        self.rollback(mark, env);
        for (p, old) in f.params.iter().zip(shadowed).rev() {
            restore(env, p, old);
        }
        r
    }
}

const MAX_DEPTH: usize = 500;
