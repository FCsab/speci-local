//! Arithmetic and comparisons.

use super::{Eval, R, Value, last_index};
use crate::parser::{BinOp, Expr};
use std::cmp::Ordering;

impl Eval<'_> {
    pub(super) fn arith(
        &self,
        e: &Expr,
        op: BinOp,
        a: &Expr,
        b: &Expr,
        x: Value,
        y: Value,
    ) -> R<Value> {
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
                .doc("ch04-01-operators.html")
                .explain(format!("Its value is {}.", v.short())));
        }
        let overflow = || {
            self.err(e, "Number overflow")
                .doc("ch04-01-operators.html")
                .explain("The result does not fit in a 64-bit whole number.")
        };
        Ok(match (op, &x, &y) {
            (BinOp::Div | BinOp::Mod | BinOp::IDiv, _, _) if y.num() == 0.0 => {
                return Err(self
                    .err(e, format!("Division by zero in `{}`", self.t(e)))
                    .doc("ch04-01-operators.html")
                    .explain(format!("`{}` is 0 here.", self.t(b))));
            }
            (BinOp::Add, Int(p), Int(q)) => Int(p.checked_add(*q).ok_or_else(overflow)?),
            (BinOp::Sub, Int(p), Int(q)) => Int(p.checked_sub(*q).ok_or_else(overflow)?),
            (BinOp::Mul, Int(p), Int(q)) => Int(p.checked_mul(*q).ok_or_else(overflow)?),
            // like the online editor (JavaScript `%`): the sign follows the dividend, -7 mod 2 = -1
            (BinOp::Mod, Int(p), Int(q)) => Int(p.checked_rem(*q).ok_or_else(overflow)?),
            (BinOp::IDiv, Int(p), Int(q)) => {
                // rounded down, like Math.floor(p / q): -7 div 2 = -4
                let d = p.checked_div(*q).ok_or_else(overflow)?;
                Int(if p % q != 0 && (*p < 0) != (*q < 0) {
                    d - 1
                } else {
                    d
                })
            }
            (BinOp::IDiv, ..) => Int((x.num() / y.num()).floor() as i64),
            (BinOp::Div, Int(p), Int(q)) if p.checked_rem(*q) == Some(0) => {
                Int(p.checked_div(*q).ok_or_else(overflow)?)
            }
            (BinOp::Add, ..) => Real(x.num() + y.num()),
            (BinOp::Sub, ..) => Real(x.num() - y.num()),
            (BinOp::Mul, ..) => Real(x.num() * y.num()),
            (BinOp::Div, ..) => Real(x.num() / y.num()),
            (BinOp::Mod, ..) => Real(x.num() % y.num()),
            _ => unreachable!(),
        })
    }

    pub(super) fn compare(
        &self,
        e: &Expr,
        op: &str,
        a: &Expr,
        b: &Expr,
        x: &Value,
        y: &Value,
    ) -> R<bool> {
        match op {
            "=" => {
                self.same_range(e, x, y)?;
                return Ok(x.eq_val(y));
            }
            "!=" => {
                self.same_range(e, x, y)?;
                return Ok(!x.eq_val(y));
            }
            "∤" => return self.compare(e, "|", a, b, x, y).map(|d| !d),
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
                        .doc("ch04-01-operators.html")
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
                    .doc("ch06-06-search.html").explain("It is a pattern result with no meaning here, e.g. the index of a SEARCH that found nothing.")
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
                    .doc("ch04-01-operators.html")
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

    /// The online editor refuses to compare arrays whose index ranges differ.
    pub(super) fn same_range(&self, e: &Expr, a: &Value, b: &Value) -> R<()> {
        if let (Value::Arr { from: f, items: x }, Value::Arr { from: g, items: y }) = (a, b)
            && (f != g || x.len() != y.len())
            && !x.is_empty()
            && !y.is_empty()
        {
            return Err(self
                .err(e, "The two arrays don't have the same index range")
                .doc("ch03-02-arrays.html").explain(format!(
                    "One is indexed {f}..{}, the other {g}..{}. Arrays are only equal if their index ranges match.",
                    last_index(*f, x.len()),
                    last_index(*g, y.len())
                )));
        }
        Ok(())
    }
}
