//! `=`: a comparison, or an assignment when one side still has no value.

use super::{Env, Eval, R, Value};
use crate::parser::{Expr, ExprKind};

impl Eval<'_> {
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

    pub(super) fn assign(&mut self, l: &Expr, r: &Expr, env: &mut Env) -> R<bool> {
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
        self.same_range(l, &a, &b)?;
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
                    Value::Int(i) => Value::Int(
                        i.checked_neg()
                            .ok_or_else(|| self.err(target, "Number overflow"))?,
                    ),
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
}
