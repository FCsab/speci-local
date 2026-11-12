//! The machine that runs the compiled steps.

use super::*;

impl Machine<'_> {
    pub(super) fn stop(&self, id: u64, what: &Expr, d: Diagnostic) -> Diagnostic {
        if self.stopped_at.get().is_some() {
            return d;
        }
        self.stopped_at.set(Some(id));
        Diagnostic::plain(format!("`{}`: {}", self.run.spec.text(what), d.title))
    }

    pub(super) fn tick(&self) -> Result<(), Diagnostic> {
        let n = self.steps.get() + 1;
        self.steps.set(n);
        if n > MAX_STEPS {
            return Err(Diagnostic::plain(
                "The structogram doesn't stop: after 2 million steps it is still running (a loop probably never ends)",
            ));
        }
        if n.is_multiple_of(1024) && self.cancel.load(Ordering::Relaxed) {
            return Err(Diagnostic::plain("Cancelled"));
        }
        Ok(())
    }

    pub(super) fn routine(&self, r: &Routine, env: &mut Env) -> Result<Option<Value>, Diagnostic> {
        let calls: &Calls = &|name, args, caller| {
            let r = self.run.routines.get(name)?;
            Some(self.call(name, r, args, caller))
        };
        let mut ev = Eval::with_cancel(&self.run.spec, self.cancel);
        ev.calls = Some(calls);
        ev.compare_only = true;
        self.block(&r.body, &mut ev, env)?;
        match &r.ret {
            Some(e) => ev.eval(e, env).map(Some),
            None => Ok(None),
        }
    }

    pub(super) fn call(
        &self,
        name: &str,
        r: &Routine,
        args: &[Value],
        caller: &Env,
    ) -> Result<Value, Diagnostic> {
        if self.depth.get() >= MAX_CALLS {
            return Err(Diagnostic::plain(format!(
                "`{name}` calls itself too deeply: the recursion probably never stops"
            )));
        }
        let spec = &self.run.spec;
        let mut env: Env = caller
            .iter()
            .filter(|(n, _)| spec.decl(n).is_some())
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect();
        for (p, v) in r.params.iter().zip(args) {
            env.insert(p.clone(), v.clone());
        }
        self.depth.set(self.depth.get() + 1);
        let result = self.routine(r, &mut env);
        self.depth.set(self.depth.get() - 1);
        match result? {
            Some(Value::Tuple(items, _)) if items.len() == r.names.len() => {
                Ok(Value::Tuple(items, r.names))
            }
            Some(v) => Ok(v),
            None => Err(Diagnostic::plain(format!(
                "The structogram of `{name}` has no return value"
            ))),
        }
    }

    pub(super) fn block(&self, ops: &[Op], ev: &mut Eval, env: &mut Env) -> Result<(), Diagnostic> {
        for op in ops {
            self.tick()?;
            match op {
                Op::Assign { id, target, value } => {
                    let v = ev.eval(value, env).map_err(|d| self.stop(*id, value, d))?;
                    self.store(target, v, ev, env)
                        .map_err(|d| self.stop(*id, target, d))?;
                }
                Op::If { id, cond, yes, no } => {
                    let c = ev.truth(cond, env).map_err(|d| self.stop(*id, cond, d))?;
                    self.block(if c { yes } else { no }, ev, env)?;
                }
                Op::Count {
                    id,
                    var,
                    lo,
                    hi,
                    body,
                } => {
                    let a = self.int(lo, ev, env).map_err(|d| self.stop(*id, lo, d))?;
                    let b = self.int(hi, ev, env).map_err(|d| self.stop(*id, hi, d))?;
                    for i in a..=b {
                        self.tick()?;
                        env.insert(var.clone(), Value::Int(i));
                        self.block(body, ev, env)?;
                    }
                }
                Op::While { id, cond, body } => {
                    while ev.truth(cond, env).map_err(|d| self.stop(*id, cond, d))? {
                        self.tick()?;
                        self.block(body, ev, env)?;
                    }
                }
                Op::Repeat { id, cond, body } => loop {
                    self.tick()?;
                    self.block(body, ev, env)?;
                    if !ev.truth(cond, env).map_err(|d| self.stop(*id, cond, d))? {
                        break;
                    }
                },
                Op::Switch { id, cases } => {
                    let mut chosen = None;
                    for (cond, body) in cases {
                        let holds = match cond {
                            Some(c) => ev.truth(c, env).map_err(|d| self.stop(*id, c, d))?,
                            None => true,
                        };
                        if holds {
                            chosen = Some(body);
                            break;
                        }
                    }
                    match chosen {
                        Some(body) => self.block(body, ev, env)?,
                        None => {
                            self.stopped_at.set(self.stopped_at.get().or(Some(*id)));
                            return Err(Diagnostic::plain(
                                "None of the cases of the multi-way decision holds",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn int(&self, e: &Expr, ev: &mut Eval, env: &mut Env) -> Result<i64, Diagnostic> {
        match ev.eval(e, env)? {
            Value::Int(i) => Ok(i),
            Value::Real(r) if r.fract() == 0.0 => Ok(r as i64),
            v => Err(Diagnostic::plain(format!(
                "must be a whole number, but it is {}",
                v.short()
            ))),
        }
    }

    pub(super) fn store(
        &self,
        target: &Expr,
        v: Value,
        ev: &mut Eval,
        env: &mut Env,
    ) -> Result<(), Diagnostic> {
        if let ExprKind::Tuple(slots) = &*target.kind {
            let Value::Tuple(items, _) = v else {
                return Err(Diagnostic::plain(format!(
                    "the value is not a tuple: {}",
                    v.short()
                )));
            };
            if items.len() != slots.len() {
                return Err(Diagnostic::plain(format!(
                    "{} variables for {} values",
                    slots.len(),
                    items.len()
                )));
            }
            for (slot, item) in slots.iter().zip(items) {
                if let Some(slot) = slot {
                    self.store(slot, item, ev, env)?;
                }
            }
            return Ok(());
        }
        let mut path = vec![];
        let root = self.path(target, ev, env, &mut path)?;
        let starts = if env.contains_key(root) {
            vec![]
        } else {
            self.starts(root, ev, env)
        };
        let mut at = env.entry(root.to_string()).or_insert(Value::Undef);
        let mut dim = 0;
        for access in path {
            match access {
                Access::Index(i) => {
                    if matches!(at, Value::Undef) {
                        *at = Value::Arr {
                            from: starts.get(dim).copied().unwrap_or(1),
                            items: Arc::new(vec![]),
                        };
                    }
                    let Value::Arr { from, items } = at else {
                        return Err(Diagnostic::plain(format!(
                            "`{root}` is {}, it can't be indexed",
                            at.kind()
                        )));
                    };
                    let k = i
                        .checked_sub(*from)
                        .and_then(|k| usize::try_from(k).ok())
                        .filter(|k| k.saturating_sub(items.len()) < MAX_GROWTH)
                        .ok_or_else(|| {
                            Diagnostic::plain(format!(
                                "index {i} is out of range (the array starts at {from})"
                            ))
                        })?;
                    let items = Arc::make_mut(items);
                    if k >= items.len() {
                        items.resize(k + 1, Value::Undef);
                    }
                    at = &mut items[k];
                    dim += 1;
                }
                Access::Field(f) => {
                    if matches!(at, Value::Undef) {
                        *at = Value::Rec(Arc::new(vec![]));
                    }
                    let Value::Rec(fields) = at else {
                        return Err(Diagnostic::plain(format!(
                            "`{root}` is {}, it has no fields",
                            at.kind()
                        )));
                    };
                    let fields = Arc::make_mut(fields);
                    let k = match fields.iter().position(|(n, _)| *n == f) {
                        Some(k) => k,
                        None => {
                            fields.push((f, Value::Undef));
                            fields.len() - 1
                        }
                    };
                    at = &mut fields[k].1;
                }
            }
        }
        *at = v;
        Ok(())
    }

    pub(super) fn path<'e>(
        &self,
        e: &'e Expr,
        ev: &mut Eval,
        env: &mut Env,
        out: &mut Vec<Access>,
    ) -> Result<&'e str, Diagnostic> {
        match &*e.kind {
            ExprKind::Var(n) => Ok(n),
            ExprKind::Index(base, idx) => {
                let root = self.path(base, ev, env, out)?;
                for i in idx {
                    out.push(Access::Index(self.int(i, ev, env)?));
                }
                Ok(root)
            }
            ExprKind::Field(base, f) => {
                let root = self.path(base, ev, env, out)?;
                out.push(Access::Field(f.clone()));
                Ok(root)
            }
            _ => Err(Diagnostic::plain("this can't be given a value")),
        }
    }

    pub(super) fn starts(&self, name: &str, ev: &mut Eval, env: &mut Env) -> Vec<i64> {
        let spec = &self.run.spec;
        let mut starts = vec![];
        let mut ty = spec.decl(name).and_then(|d| spec.resolve(&d.ty));
        while let Some(Type::Array { lo, elem, .. }) = ty {
            starts.push(match ev.eval(lo, env) {
                Ok(Value::Int(i)) => i,
                _ => 1,
            });
            ty = spec.resolve(elem);
        }
        starts
    }
}
