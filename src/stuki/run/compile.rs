//! Compiling the texts of a structogram into steps the machine runs.

use super::*;

impl Compiler<'_> {
    pub(super) fn fragment(&mut self, text: &str, id: u64) -> Result<Fragment, Stop> {
        let from = self.src.len();
        self.src.push_str(text);
        self.src.push('\n');
        let f = parse_fragment(&self.src, from, from + text.len()).map_err(|d| {
            (
                format!("`{}` can't be run: {}", text.trim(), d.title),
                Some(id),
            )
        })?;
        let parts = match &f {
            Fragment::Assign(a, b) | Fragment::Range(_, a, b) => vec![a, b],
            Fragment::Expr(e) => vec![e],
        };
        for e in parts {
            self.calls(e).map_err(|m| (m, Some(id)))?;
        }
        Ok(f)
    }

    pub(super) fn calls(&self, e: &Expr) -> Result<(), String> {
        if let ExprKind::Call(name, args) = &*e.kind {
            let want = BUILTINS
                .iter()
                .find(|b| b.0 == name)
                .map(|b| b.1)
                .or_else(|| self.arity.get(name).copied())
                .or_else(|| self.spec.funcs.get(name).map(|f| f.params.len()));
            match want {
                None => {
                    return Err(format!(
                        "`{name}` is not a function of the specification or a structogram"
                    ));
                }
                Some(n) if n != args.len() => {
                    return Err(format!(
                        "`{name}` takes {n} argument(s) but gets {} here",
                        args.len()
                    ));
                }
                Some(_) => {}
            }
        }
        e.children().into_iter().try_for_each(|c| self.calls(c))
    }

    pub(super) fn cond(&mut self, text: &str, id: u64) -> Result<Expr, Stop> {
        match self.fragment(text, id)? {
            Fragment::Expr(e) => Ok(e),
            _ => Err((format!("`{}` is not a condition", text.trim()), Some(id))),
        }
    }

    pub(super) fn block(&mut self, nodes: &[Node]) -> Result<Vec<Op>, Stop> {
        let mut ops = vec![];
        for n in nodes {
            let id = n.id;
            match &n.kind {
                Kind::Noop => {}
                Kind::Statement(t) if matches!(t.text.trim(), "" | "-") => {}
                // `Be: n, x` and `Ki: s, db` only show the inputs and the outputs
                Kind::Statement(t) if is_io(&t.text) => {}
                Kind::Statement(t) => match self.fragment(&t.text, id)? {
                    Fragment::Assign(target, value) => {
                        target_ok(&target).map_err(|m| (m, Some(id)))?;
                        ops.push(Op::Assign { id, target, value });
                    }
                    _ => {
                        return Err((
                            format!(
                                "`{}` is not a step: a statement must give a variable its value, like `x:=…`",
                                t.text.trim()
                            ),
                            Some(id),
                        ));
                    }
                },
                Kind::If { cond, yes, no } => ops.push(Op::If {
                    id,
                    cond: self.cond(&cond.text, id)?,
                    yes: self.block(yes)?,
                    no: self.block(no)?,
                }),
                Kind::While { cond, body } => {
                    let head = self.fragment(&cond.text, id)?;
                    let body = self.block(body)?;
                    ops.push(match head {
                        Fragment::Range(var, lo, hi) => Op::Count {
                            id,
                            var,
                            lo,
                            hi,
                            body,
                        },
                        Fragment::Expr(cond) => Op::While { id, cond, body },
                        Fragment::Assign(..) => {
                            return Err((
                                format!("`{}` is not a condition", cond.text.trim()),
                                Some(id),
                            ));
                        }
                    });
                }
                Kind::Repeat { cond, body } => ops.push(Op::Repeat {
                    id,
                    cond: self.cond(&cond.text, id)?,
                    body: self.block(body)?,
                }),
                Kind::Switch { cases } => {
                    let mut compiled = vec![];
                    for c in cases {
                        let cond = match &c.cond {
                            Some(t) => Some(self.cond(&t.text, id)?),
                            None => None,
                        };
                        compiled.push((cond, self.block(&c.body)?));
                    }
                    ops.push(Op::Switch {
                        id,
                        cases: compiled,
                    });
                }
            }
        }
        Ok(ops)
    }

    pub(super) fn routine(
        &mut self,
        d: &Diagram,
        function: Option<(&str, Vec<String>)>,
    ) -> Result<Routine, Stop> {
        let body = self.block(&d.body)?;
        let Some((name, params)) = function else {
            return Ok(Routine {
                params: vec![],
                body,
                ret: None,
                names: &[],
            });
        };
        let ret = d.ret.text.trim();
        let ret = if d.show_return && !ret.is_empty() {
            Some(self.cond(ret, d.id).map_err(|(m, _)| (m, None))?)
        } else {
            None
        };
        let names = match self.spec.funcs.get(name).map(|f| &*f.body.kind) {
            Some(ExprKind::Macro { name, .. }) => result_names(name),
            _ => &[],
        };
        Ok(Routine {
            params,
            body,
            ret,
            names,
        })
    }
}

/// `Be: n, x` / `In: n, x` and `Ki: s, db` / `Out: s, db`: the statements Auto Stuki starts and
/// ends with, showing the inputs and the outputs.
pub(super) fn is_io(text: &str) -> bool {
    let t = text.trim_start();
    ["Be:", "In:", "Ki:", "Out:"]
        .iter()
        .any(|k| t.starts_with(k))
}

pub(super) fn target_ok(e: &Expr) -> Result<(), String> {
    match &*e.kind {
        ExprKind::Var(_) => Ok(()),
        ExprKind::Index(base, _) | ExprKind::Field(base, _) => target_ok(base),
        ExprKind::Tuple(slots) => slots.iter().flatten().try_for_each(target_ok),
        _ => {
            Err("Only a variable, an array element or a record's field can be given a value".into())
        }
    }
}
