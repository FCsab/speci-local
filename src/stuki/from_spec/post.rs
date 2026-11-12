//! The postcondition: its parts become assignments, loops and decisions.

use super::*;

impl Gen<'_> {
    /// An output (or auxiliary variable) that has no value yet.
    pub(super) fn target(&self, e: &Expr) -> Option<String> {
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
    pub(super) fn targets(&self, e: &Expr) -> Option<Vec<Option<String>>> {
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

    /// `y[…]`: an element of an output array that has no value yet, or is given one element by
    /// element.
    pub(super) fn element(&self, e: &Expr) -> Option<String> {
        let ExprKind::Index(base, _) = &*e.kind else {
            return None;
        };
        match &*base.kind {
            ExprKind::Var(n)
                if self
                    .spec
                    .decl(n)
                    .is_some_and(|d| d.role != Role::In && d.init.is_none())
                    && (!self.assigned.contains(n) || self.elems.contains(n)) =>
            {
                Some(n.clone())
            }
            _ => None,
        }
    }

    /// The outputs a part of the postcondition sets, and the ones it reads.
    pub(super) fn sets_and_reads(&self, c: &Expr) -> (Vec<String>, Vec<String>) {
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
                if let Some(n) = self.element(t) {
                    // `y[i]=y[i-1]+x[i]` reads elements set before
                    return (vec![n], outputs(c));
                }
            }
        }
        if let ExprKind::Quant {
            all: true, body, ..
        } = &*c.kind
        {
            let sets = self.sets_of(body);
            if !sets.is_empty() {
                let reads = outputs(c)
                    .into_iter()
                    .filter(|n| !sets.contains(n))
                    .collect();
                return (sets, reads);
            }
        }
        (vec![], outputs(c))
    }

    /// The steps of the parts of the postcondition (or of a `∀` loop's body).
    pub(super) fn block(&mut self, parts: &[&Expr]) -> Vec<Node> {
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
            self.part(k, parts, &mut done, &mut out);
        }
        // the rest in the order written; the plain conditions are left out there
        for k in 0..parts.len() {
            if !done[k] {
                self.part(k, parts, &mut done, &mut out);
            }
        }
        out
    }

    /// Part `k` of the postcondition. `c -> …` takes a `not c -> …` along as its else branch.
    /// A part that sets no output is a condition on the result: it needs no steps.
    pub(super) fn part(
        &mut self,
        k: usize,
        parts: &[&Expr],
        done: &mut [bool],
        out: &mut Vec<Node>,
    ) {
        done[k] = true;
        if stray(parts[k], true) || self.sets_and_reads(parts[k]).0.is_empty() {
            return;
        }
        let ExprKind::Bin(BinOp::Imp, a, b) = &*parts[k].kind else {
            return self.step(parts[k], out);
        };
        let others: Vec<(usize, &Expr, &Expr)> = (0..parts.len())
            .filter(|&j| !done[j] && !stray(parts[j], true))
            .filter_map(|j| match &*parts[j].kind {
                ExprKind::Bin(BinOp::Imp, a2, b2) => Some((j, a2, b2)),
                _ => None,
            })
            .collect();
        if let Some(&(j, _, b2)) = others.iter().find(|(_, a2, _)| self.opposite(a, a2)) {
            done[j] = true;
            return self.decision(a, b, Some(b2), out);
        }
        let mine = self.sets_of(b);
        let decides = |g: &Self, c: &Expr| {
            conjuncts(c)
                .into_iter()
                .all(|p| g.sets_and_reads(p).0.is_empty())
        };
        let same: Vec<_> = others
            .into_iter()
            .filter(|(_, a2, b2)| {
                decides(self, a2) && self.sets_of(b2).iter().any(|s| mine.contains(s))
            })
            .collect();
        if same.is_empty() || !decides(self, a) {
            return self.decision(a, b, None, out);
        }
        let mut arms = vec![(a, b)];
        for (j, a2, b2) in same {
            done[j] = true;
            arms.push((a2, b2));
        }
        self.multiway(&arms, out);
    }

    pub(super) fn sets_of(&self, e: &Expr) -> Vec<String> {
        conjuncts(e)
            .into_iter()
            .flat_map(|p| self.sets_and_reads(p).0)
            .collect()
    }

    /// `c` and `nem c` (either way round).
    pub(super) fn opposite(&self, a: &Expr, b: &Expr) -> bool {
        match (&*a.kind, &*b.kind) {
            (ExprKind::Not(x), _) if self.text(x) == self.text(b) => true,
            (_, ExprKind::Not(y)) if self.text(a) == self.text(y) => true,
            (ExprKind::Cmp(v, p), ExprKind::Cmp(u, q)) if v.len() == 2 && u.len() == 2 => {
                let same = |x: &Expr, y: &Expr| self.text(x) == self.text(y);
                let straight = same(&v[0], &u[0]) && same(&v[1], &u[1]);
                let swapped = same(&v[0], &u[1]) && same(&v[1], &u[0]);
                straight && negated(p[0]) == Some(q[0])
                    || swapped && mirrored(p[0]).and_then(negated) == Some(q[0])
            }
            _ => false,
        }
    }

    pub(super) fn multiway(&mut self, arms: &[(&Expr, &Expr)], out: &mut Vec<Node>) {
        let before = self.assigned.clone();
        let mut after = before.clone();
        let mut cases = vec![];
        for (a, b) in arms {
            self.assigned = before.clone();
            let mut body = vec![];
            for p in conjuncts(b) {
                self.step(p, &mut body);
            }
            after.extend(self.assigned.drain());
            let cond = conjuncts(a)
                .into_iter()
                .map(|c| self.cond_value(c, out))
                .collect::<Vec<_>>()
                .join(&format!(" {} ", self.w.and));
            cases.push(Case {
                id: fresh_id(),
                cond: Some(Rich::plain(&cond)),
                body: seq(body),
            });
        }
        self.assigned = after;
        out.push(node(Kind::Switch { cases }));
    }

    /// `c -> x=…`: a decision. Equations in `c` that set outputs are done first, as steps.
    pub(super) fn decision(
        &mut self,
        a: &Expr,
        b: &Expr,
        otherwise: Option<&Expr>,
        out: &mut Vec<Node>,
    ) {
        let mut conds = vec![];
        for p in conjuncts(a) {
            if self.sets_and_reads(p).0.is_empty() {
                conds.push(p);
            } else {
                self.step(p, out);
            }
        }
        // `van -> (van,,m)=FELTMAX(…)`: what sets the condition's own variables runs first
        let reads = vars_in(a);
        let (first, then): (Vec<_>, Vec<_>) = conjuncts(b)
            .into_iter()
            .partition(|p| self.sets_and_reads(p).0.iter().any(|s| reads.contains(s)));
        // each branch may set the same outputs, the else branch also what runs first
        // (`nem van -> m=0` after the FELTMAX above)
        let before = self.assigned.clone();
        for p in first {
            self.step(p, out);
        }
        let mut yes = vec![];
        for p in then {
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
            .map(|c| self.cond_value(c, out))
            .collect::<Vec<_>>()
            .join(&format!(" {} ", self.w.and));
        out.push(node(Kind::If {
            cond: Rich::plain(&cond),
            yes: seq(yes),
            no: seq(no),
        }));
    }

    /// One part of the postcondition.
    pub(super) fn step(&mut self, c: &Expr, out: &mut Vec<Node>) {
        if let Some((l, r)) = equation(c) {
            // `egyeb=egyeb`: only there to mention a variable
            if self.text(l) == self.text(r) {
                return;
            }
            for (t, v) in [(l, r), (r, l)] {
                if let Some(ts) = self.targets(t) {
                    self.assign(t, &ts, v, out);
                    self.assigned.extend(ts.into_iter().flatten());
                    return;
                }
                if let Some(n) = self.element(t) {
                    let v = self.value(v, out);
                    out.push(stmt(format!("{}:={v}", self.text(t))));
                    self.assigned.insert(n.clone());
                    self.elems.insert(n);
                    return;
                }
            }
        }
        // `∀i∈[a..b]:(…)` that sets outputs: a counting loop of the steps of its body
        if let ExprKind::Quant {
            all: true,
            var,
            lo,
            hi,
            body,
        } = &*c.kind
            && !self.sets_of(body).is_empty()
        {
            self.local(var, self.w.int);
            self.active.push(var.clone());
            let steps = self.block(&conjuncts(body));
            self.active.pop();
            out.push(while_loop(
                format!("{var}={}..{}", self.text(lo), self.text(hi)),
                steps,
            ));
        }
        // anything else sets nothing: a condition on the result, which needs no steps
    }

    pub(super) fn assign(
        &mut self,
        t: &Expr,
        ts: &[Option<String>],
        v: &Expr,
        out: &mut Vec<Node>,
    ) {
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
}
