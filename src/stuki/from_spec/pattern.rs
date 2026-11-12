//! The standard algorithms of the patterns and quantifiers.

use super::*;

impl Gen<'_> {
    /// A new local variable for a pattern's single result.
    pub(super) fn temp(&mut self, name: &str, args: &[Expr]) -> String {
        self.fresh_slots(name, args).remove(0)
    }

    /// The results of a pattern: (variable name to use, type), in the order of its tuple.
    pub(super) fn slots(&self, name: &str, args: &[Expr]) -> Vec<(&'static str, String)> {
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
    pub(super) fn fresh_slots(&mut self, name: &str, args: &[Expr]) -> Vec<String> {
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
    pub(super) fn pattern(
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
        let lo_t = self.text(lo);
        let hi_t = hi.map(|h| self.text(h)).unwrap_or_default();
        let theirs = var;
        let var = if self.active.iter().any(|a| a == theirs) {
            self.fresh(theirs)
        } else {
            theirs.to_string()
        };
        let var = var.as_str();
        self.local(var, w.int);
        self.active.push(var.to_string());
        if var != theirs {
            self.renames.push((theirs.to_string(), var.to_string()));
        }
        let range = format!("{var}={lo_t}..{hi_t}");
        let set = |x: &str, v: &str| stmt(format!("{x}:={v}"));
        let step = || set(var, &format!("{var}+1"));
        match name {
            "SUM" => {
                out.push(set(&t[0], "0"));
                let mut body = vec![];
                match args.get(1) {
                    Some(c) => {
                        let cv = self.cond_value(c, &mut body);
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
                let cv = self.cond_value(&args[0], &mut body);
                body.push(if_then(cv, vec![set(&t[0], &format!("{}+1", t[0]))]));
                out.push(while_loop(range, body));
            }
            "EXISTS" | "FORALL" | "SEARCH" if contains_lift(&args[0]) => {
                let all = name == "FORALL";
                out.push(set(&t[0], if all { w.yes } else { w.no }));
                out.push(set(var, &lo_t));
                let mut body = vec![];
                let cv = self.value(&args[0], &mut body);
                body.push(set(&t[0], &cv));
                body.push(step());
                let go = if all {
                    t[0].clone()
                } else {
                    format!("{} {}", w.not, t[0])
                };
                out.push(while_loop(format!("{var}≤{hi_t} {} {go}", w.and), body));
                if name == "SEARCH" {
                    out.push(if_then(&t[0], vec![set(&t[1], &format!("{var}-1"))]));
                }
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
            "SELECT" if contains_lift(&args[0]) => {
                let found = self.fresh(w.found);
                self.local(&found, w.boolean);
                out.push(set(&found, w.no));
                out.push(set(var, &self.shifted(lo, -1)));
                let mut body = vec![step()];
                let cv = self.value(&args[0], &mut body);
                body.push(set(&found, &cv));
                out.push(while_loop(format!("{} {found}", w.not), body));
                out.push(set(&t[0], var));
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
                let cv = self.cond_value(&args[0], &mut body);
                let mut yes = vec![set(&t[0], &format!("{}+1", t[0]))];
                let item = match args.get(1) {
                    Some(f) => self.value(f, &mut yes),
                    None => var.to_string(),
                };
                yes.push(set(&format!("{}[{}]", t[1], t[0]), &item));
                body.push(if_then(cv, yes));
                out.push(while_loop(range, body));
            }
            "MAX" | "MIN" if contains_lift(&args[0]) => {
                let rel = if name == "MAX" { ">" } else { "<" };
                let mut body = vec![];
                let fv = self.value(&args[0], &mut body);
                body.push(if_then(
                    format!("{var}={lo_t} {} {fv}{rel}{}", w.or, t[1]),
                    vec![set(&t[0], var), set(&t[1], &fv)],
                ));
                out.push(while_loop(range, body));
            }
            "MAX" | "MIN" => {
                let rel = if name == "MAX" { ">" } else { "<" };
                out.push(set(&t[0], &lo_t));
                out.push(set(&t[1], &self.subst(&args[0], theirs, &lo_t)));
                let mut body = vec![];
                let fv = self.value(&args[0], &mut body);
                body.push(if_then(
                    format!("{fv}{rel}{}", t[1]),
                    vec![set(&t[0], var), set(&t[1], &fv)],
                ));
                out.push(while_loop(
                    format!("{var}={}..{hi_t}", self.shifted(lo, 1)),
                    body,
                ));
            }
            _ => {
                // CONDMAX / CONDMIN: the three cases of the ELTE algorithm
                let rel = if name == "CONDMAX" { ">" } else { "<" };
                let (f, c) = (&args[0], &args[1]);
                out.push(set(&t[0], w.no));
                let mut body = vec![];
                let (cond, not_cond) = if contains_lift(c) {
                    let cv = self.value(c, &mut body);
                    let whole = match &*c.kind {
                        ExprKind::Bin(BinOp::Or | BinOp::Imp, ..) => format!("({cv})"),
                        _ => cv.clone(),
                    };
                    (whole, format!("{} ({cv})", w.not))
                } else {
                    (self.conj_part(c), self.negate(c))
                };
                let fv = self.value(f, &mut body);
                let found = &t[0];
                let case = |cond: String, body: Vec<Node>| Case {
                    id: fresh_id(),
                    cond: Some(Rich::plain(&cond)),
                    body: seq(body),
                };
                let cases = vec![
                    case(not_cond, vec![]),
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
        self.active.pop();
        if var != theirs {
            self.renames.pop();
        }
    }
}
