//! The theorem macros (programming patterns): SUM, COUNT, MAX, SEARCH, SELECT, COPY, MIS, …

use super::value::set;
use super::{Env, Eval, R, Value};
use crate::parser::Expr;
use std::cmp::Ordering;

impl Eval<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn pattern(
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
                set(env, var, Value::Int(i));
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
        let at = |env: &mut Env, i: i64| {
            set(env, var, Value::Int(i));
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
                    at(env, i);
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
                    at(env, i);
                    n += self.truth(&args[0], env)? as i64;
                }
                Ok(Value::Int(n))
            }
            "EXISTS" | "FORALL" => {
                let all = name == "FORALL";
                for i in range {
                    at(env, i);
                    if self.truth(&args[0], env)? != all {
                        return Ok(Value::Bool(!all));
                    }
                }
                Ok(Value::Bool(all))
            }
            "SEARCH" => {
                for i in range {
                    at(env, i);
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
                    at(env, i);
                    items.push(self.eval(&args[0], env)?);
                }
                Ok(Value::Arr {
                    from: b,
                    items: items.into(),
                })
            }
            "MIS" => {
                let mut items = Vec::new();
                for i in range {
                    at(env, i);
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
                        Value::Arr {
                            from: 1,
                            items: items.into(),
                        },
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
                    at(env, i);
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
