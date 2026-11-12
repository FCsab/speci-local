//! Running the structograms on a test case of the specification.

use super::model::{Diagram, Kind, Node, Program};
use crate::data;
use crate::diag::Diagnostic;
use crate::eval::{Calls, Env, Eval, Value, result_names};
use crate::parser::{BUILTINS, Expr, ExprKind, Fragment, Role, Spec, Type, parse_fragment};
use crate::runner::explain_false;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_STEPS: u64 = 2_000_000;
const MAX_CALLS: u32 = 100;
const MAX_GROWTH: usize = 1_000_000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Verdict {
    pub pass: bool,
    pub message: String,
    pub node: Option<u64>,
    pub values: Vec<(String, String)>,
}

impl Verdict {
    fn failed(message: impl Into<String>, node: Option<u64>) -> Verdict {
        Verdict {
            message: message.into(),
            node,
            ..Verdict::default()
        }
    }
}

enum Op {
    Assign {
        id: u64,
        target: Expr,
        value: Expr,
    },
    If {
        id: u64,
        cond: Expr,
        yes: Vec<Op>,
        no: Vec<Op>,
    },
    Count {
        id: u64,
        var: String,
        lo: Expr,
        hi: Expr,
        body: Vec<Op>,
    },
    While {
        id: u64,
        cond: Expr,
        body: Vec<Op>,
    },
    Repeat {
        id: u64,
        cond: Expr,
        body: Vec<Op>,
    },
    Switch {
        id: u64,
        cases: Vec<(Option<Expr>, Vec<Op>)>,
    },
}

struct Routine {
    params: Vec<String>,
    body: Vec<Op>,
    ret: Option<Expr>,
    names: &'static [&'static str],
}

type Stop = (String, Option<u64>);

fn header(name: &str) -> Option<(String, Vec<String>)> {
    let name = name.trim();
    let open = name.find('(')?;
    let word = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_');
    let f = name[..open].trim();
    let mut depth = 0;
    let close = name[open..].char_indices().find_map(|(i, c)| {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
        (depth == 0).then_some(open + i)
    })?;
    let inside = &name[open + 1..close];
    let mut params = vec![];
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in inside.char_indices().chain([(inside.len(), ',')]) {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                let p = inside[start..i].split(':').next().unwrap_or("").trim();
                if !p.is_empty() || i < inside.len() || !params.is_empty() {
                    params.push(p.to_string());
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    (word(f) && params.iter().all(|p| word(p))).then(|| (f.to_string(), params))
}

struct Compiler<'a> {
    spec: &'a Spec,
    src: String,
    arity: HashMap<String, usize>,
}

impl Compiler<'_> {
    fn fragment(&mut self, text: &str, id: u64) -> Result<Fragment, Stop> {
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

    fn calls(&self, e: &Expr) -> Result<(), String> {
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

    fn cond(&mut self, text: &str, id: u64) -> Result<Expr, Stop> {
        match self.fragment(text, id)? {
            Fragment::Expr(e) => Ok(e),
            _ => Err((format!("`{}` is not a condition", text.trim()), Some(id))),
        }
    }

    fn block(&mut self, nodes: &[Node]) -> Result<Vec<Op>, Stop> {
        let mut ops = vec![];
        for n in nodes {
            let id = n.id;
            match &n.kind {
                Kind::Noop => {}
                Kind::Statement(t) if matches!(t.text.trim(), "" | "-") => {}
                // `Ki: s, db` only shows the outputs
                Kind::Statement(t) if is_output(&t.text) => {}
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

    fn routine(
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

/// `Ki: s, db` / `Out: s, db`: the statement Auto Stuki ends with, showing the outputs.
fn is_output(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("Ki:") || t.starts_with("Out:")
}

fn target_ok(e: &Expr) -> Result<(), String> {
    match &*e.kind {
        ExprKind::Var(_) => Ok(()),
        ExprKind::Index(base, _) | ExprKind::Field(base, _) => target_ok(base),
        ExprKind::Tuple(slots) => slots.iter().flatten().try_for_each(target_ok),
        _ => {
            Err("Only a variable, an array element or a record's field can be given a value".into())
        }
    }
}

enum Access {
    Index(i64),
    Field(String),
}

pub struct Runner {
    spec: Spec,
    main: Routine,
    routines: HashMap<String, Routine>,
}

struct Machine<'a> {
    run: &'a Runner,
    cancel: &'a AtomicBool,
    steps: Cell<u64>,
    depth: Cell<u32>,
    stopped_at: Cell<Option<u64>>,
}

impl Runner {
    pub fn new(program: &Program, spec: &Spec) -> Result<Runner, Verdict> {
        let headers: Vec<_> = program
            .diagrams
            .iter()
            .map(|d| header(&d.name.text))
            .collect();
        let main = headers
            .iter()
            .position(Option::is_none)
            .or((!headers.is_empty()).then_some(0))
            .ok_or_else(|| Verdict::failed("There is no structogram to run", None))?;
        let mut c = Compiler {
            spec,
            src: format!("{}\n", spec.src),
            arity: HashMap::new(),
        };
        for (k, h) in headers.iter().enumerate() {
            if let Some((name, params)) = h
                && k != main
            {
                c.arity.insert(name.clone(), params.len());
            }
        }
        let stop = |(message, node): Stop| Verdict::failed(message, node);
        let mut routines = HashMap::new();
        let mut main_routine = None;
        for (k, (d, h)) in program.diagrams.iter().zip(&headers).enumerate() {
            if k == main {
                main_routine = Some(c.routine(d, None).map_err(stop)?);
            } else if let Some((name, params)) = h {
                let r = c
                    .routine(d, Some((name.as_str(), params.clone())))
                    .map_err(stop)?;
                routines.insert(name.clone(), r);
            }
        }
        Ok(Runner {
            spec: Spec {
                src: c.src,
                ..spec.clone()
            },
            main: main_routine.expect("the main structogram"),
            routines,
        })
    }

    pub fn test(&self, data: &str, cancel: &AtomicBool) -> Verdict {
        let spec = &self.spec;
        let given = match data::load(spec, data) {
            Ok(env) => env,
            Err(d) => return Verdict::failed(d.title, None),
        };
        let mut ev = Eval::with_cancel(spec, cancel);
        match ev.truth(&spec.pre, &mut given.clone()) {
            Err(d) => return Verdict::failed(d.title, None),
            Ok(false) => {
                return Verdict::failed(
                    "The test case's input doesn't satisfy the precondition",
                    None,
                );
            }
            Ok(true) => {}
        }
        let mut env: Env = given
            .into_iter()
            .filter(|(name, _)| {
                spec.decl(name)
                    .is_some_and(|d| d.role == Role::In || d.init.is_some())
            })
            .collect();
        let machine = Machine {
            run: self,
            cancel,
            steps: Cell::new(0),
            depth: Cell::new(0),
            stopped_at: Cell::new(None),
        };
        if let Err(d) = machine.routine(&self.main, &mut env) {
            return Verdict::failed(d.title, machine.stopped_at.get());
        }
        env.retain(|name, _| spec.decl(name).is_some());
        let outputs = || spec.decls.iter().filter(|d| d.role != Role::In);
        let values = outputs()
            .map(|d| {
                let v = env
                    .get(&d.name)
                    .map_or_else(|| "— (no value)".into(), Value::summary);
                (d.name.clone(), v)
            })
            .collect();
        let which = format!("postcondition ({})", spec.post_kw.trim_end_matches(':'));
        let mut ev = Eval::with_cancel(spec, cancel);
        let mut filled = env.clone();
        let message = match ev.truth(&spec.post, &mut filled) {
            Err(d) => d.title,
            Ok(false) => {
                let d = explain_false(&mut ev, &spec.post, &env, &which);
                match d.explanation.is_empty() {
                    true => d.title,
                    false => format!("{}\n{}", d.title, d.explanation),
                }
            }
            Ok(true) => outputs()
                .filter(|d| !env.contains_key(&d.name))
                .find_map(|d| match filled.get(&d.name) {
                    Some(Value::Undef) | None => None,
                    Some(Value::Arr { items, .. }) if items.is_empty() => None,
                    Some(v) => Some(format!(
                        "`{}` never gets a value (the specification gives {})",
                        d.name,
                        v.short()
                    )),
                })
                .unwrap_or_default(),
        };
        Verdict {
            pass: message.is_empty(),
            message,
            node: None,
            values,
        }
    }
}

pub fn run_tests(
    program: &Program,
    spec: &Spec,
    tests: &[String],
    cancel: &AtomicBool,
) -> Vec<Verdict> {
    match Runner::new(program, spec) {
        Ok(r) => tests.iter().map(|t| r.test(t, cancel)).collect(),
        Err(v) => vec![v; tests.len()],
    }
}

impl Machine<'_> {
    fn stop(&self, id: u64, what: &Expr, d: Diagnostic) -> Diagnostic {
        if self.stopped_at.get().is_some() {
            return d;
        }
        self.stopped_at.set(Some(id));
        Diagnostic::plain(format!("`{}`: {}", self.run.spec.text(what), d.title))
    }

    fn tick(&self) -> Result<(), Diagnostic> {
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

    fn routine(&self, r: &Routine, env: &mut Env) -> Result<Option<Value>, Diagnostic> {
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

    fn call(
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

    fn block(&self, ops: &[Op], ev: &mut Eval, env: &mut Env) -> Result<(), Diagnostic> {
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

    fn int(&self, e: &Expr, ev: &mut Eval, env: &mut Env) -> Result<i64, Diagnostic> {
        match ev.eval(e, env)? {
            Value::Int(i) => Ok(i),
            Value::Real(r) if r.fract() == 0.0 => Ok(r as i64),
            v => Err(Diagnostic::plain(format!(
                "must be a whole number, but it is {}",
                v.short()
            ))),
        }
    }

    fn store(
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

    fn path<'e>(
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

    fn starts(&self, name: &str, ev: &mut Eval, env: &mut Env) -> Vec<i64> {
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

#[cfg(test)]
mod tests {
    use super::super::from_spec::from_spec;
    use super::super::model::{Case, fresh_id};
    use super::super::rich::Rich;
    use super::*;
    use crate::parser::parse;

    const COUNT: &str = "Be:\nn∈N,\nx∈Z[1..n]\nKi:\ndb∈N\nEf:\n-\nUf:\ndb=DARAB(i=1..n, x[i]>0)\n";
    const DATA: &str = "n: 4\nx: [3, -1, 0, 8]\ndb: 2";

    fn node(kind: Kind) -> Node {
        Node {
            id: fresh_id(),
            bg: None,
            kind,
        }
    }

    fn st(text: &str) -> Node {
        node(Kind::Statement(Rich::plain(text)))
    }

    fn when(cond: &str, yes: Vec<Node>, no: Vec<Node>) -> Node {
        node(Kind::If {
            cond: Rich::plain(cond),
            yes,
            no,
        })
    }

    fn during(cond: &str, body: Vec<Node>) -> Node {
        node(Kind::While {
            cond: Rich::plain(cond),
            body,
        })
    }

    fn program(diagrams: Vec<(&str, &str, Vec<Node>)>) -> Program {
        Program {
            diagrams: diagrams
                .into_iter()
                .map(|(name, ret, body)| Diagram {
                    name: Rich::plain(name),
                    ret: Rich::plain(ret),
                    show_header: true,
                    body,
                    ..Diagram::new()
                })
                .collect(),
            ..Program::default()
        }
    }

    fn run(p: &Program, spec: &str, data: &str) -> Verdict {
        let spec = parse(spec).unwrap_or_else(|e| panic!("{}", e[0].title));
        run_tests(p, &spec, &[data.to_string()], &AtomicBool::new(false)).remove(0)
    }

    fn counting(step: &str) -> Program {
        program(vec![(
            "Számlálás",
            "",
            vec![
                st("db:=0"),
                st("i:=1"),
                during(
                    "i≤n",
                    vec![
                        when("x[i]>0", vec![st("db:=db+1")], vec![st("-")]),
                        st(step),
                    ],
                ),
            ],
        )])
    }

    #[test]
    fn a_hand_written_structogram_passes_or_fails() {
        let v = run(&counting("i:=i+1"), COUNT, DATA);
        assert!(v.pass, "{}", v.message);
        assert_eq!(v.values, [("db".to_string(), "2".to_string())]);

        let wrong = program(vec![("P", "", vec![st("db:=n")])]);
        let v = run(&wrong, COUNT, DATA);
        assert!(!v.pass);
        assert!(
            v.message.contains("postcondition (Uf) is false"),
            "{}",
            v.message
        );
        assert_eq!(v.values[0].1, "4");

        let v = run(&program(vec![("P", "", vec![st("-")])]), COUNT, DATA);
        assert_eq!(
            v.message,
            "`db` never gets a value (the specification gives 2)"
        );
    }

    #[test]
    fn the_element_that_stops_the_run_is_named() {
        let p = counting("i:=i");
        let v = run(&p, COUNT, DATA);
        assert!(
            v.message.starts_with("The structogram doesn't stop"),
            "{}",
            v.message
        );

        let p = program(vec![(
            "P",
            "",
            vec![st("db:=0"), st("db:=x[n+1]"), st("db:=1")],
        )]);
        let v = run(&p, COUNT, DATA);
        assert_eq!(v.node, Some(p.diagrams[0].body[1].id));
        assert!(
            v.message.starts_with("`x[n+1]`: ") && v.message.contains("out of range"),
            "{}",
            v.message
        );

        let v = run(&program(vec![("P", "", vec![st("db:=k+1")])]), COUNT, DATA);
        assert_eq!(v.message, "`k+1`: `k` has no value at this point");

        let v = run(
            &program(vec![("P", "", vec![when("db=0", vec![], vec![])])]),
            COUNT,
            DATA,
        );
        assert_eq!(v.message, "`db=0`: `db` has no value at this point");
    }

    #[test]
    fn texts_that_are_not_steps_are_reported_before_running() {
        let p = program(vec![(
            "P",
            "",
            vec![st("db:=0"), st("count the positives")],
        )]);
        let v = run(&p, COUNT, DATA);
        assert_eq!(v.node, Some(p.diagrams[0].body[1].id));
        assert!(v.message.contains("can't be run"), "{}", v.message);

        let v = run(&program(vec![("P", "", vec![st("db<n")])]), COUNT, DATA);
        assert!(v.message.contains("is not a step"), "{}", v.message);
        let v = run(&program(vec![("P", "", vec![st("db:=f(n)")])]), COUNT, DATA);
        assert!(v.message.contains("`f` is not a function"), "{}", v.message);
        let v = run(&program(vec![("P", "", vec![st("3:=n")])]), COUNT, DATA);
        assert!(v.message.starts_with("Only a variable"), "{}", v.message);
        let v = run(
            &program(vec![("P", "", vec![during("i:=1", vec![])])]),
            COUNT,
            DATA,
        );
        assert!(v.message.contains("is not a condition"), "{}", v.message);
        let v = run(
            &Program {
                diagrams: vec![],
                ..Program::default()
            },
            COUNT,
            DATA,
        );
        assert_eq!(v.message, "There is no structogram to run");
    }

    #[test]
    fn test_cases_the_specification_rejects_are_not_run() {
        let spec = COUNT.replace("Ef:\n-", "Ef:\nn>9");
        let v = run(&counting("i:=i+1"), &spec, DATA);
        assert_eq!(
            v.message,
            "The test case's input doesn't satisfy the precondition"
        );
        let v = run(&counting("i:=i+1"), COUNT, "n: [");
        assert!(!v.pass && !v.message.is_empty());
    }

    #[test]
    fn functions_are_called_by_their_header() {
        let spec = "Be:\nn∈N,\nx∈Z[1..n]\nKi:\ns∈Z,\nh∈N\nFv:\nketszer:Z->Z, ketszer(a)=2*a\nEf:\nn>0\nUf:\ns=SZUMMA(i=1..n, ketszer(x[i])) és (h,)=MAX(i=1..n, x[i])\n";
        let main = vec![
            st("s:=0"),
            during("i=1..n", vec![st("s:=s+ketszer(x[i])")]),
            st("(h, e):=legnagyobb(n)"),
        ];
        let largest = vec![
            st("mi:=1"),
            during(
                "i=2..k",
                vec![when("x[i]>x[mi]", vec![st("mi:=i")], vec![])],
            ),
        ];
        let p = program(vec![
            ("ketszer(a:Egész):Egész", "a+a", vec![st("-")]),
            ("Főprogram", "", main),
            ("legnagyobb(k:Egész):Egész x Egész", "(mi, x[mi])", largest),
        ]);
        let v = run(&p, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
        assert!(v.pass, "{}", v.message);
        assert_eq!(
            v.values,
            [("s".into(), "22".into()), ("h".into(), "2".to_string())]
        );

        let mut wrong = p.clone();
        wrong.diagrams[0].ret = Rich::plain("a");
        let v = run(&wrong, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
        assert!(!v.pass && v.values[0].1 == "11", "{v:?}");

        let mut endless = p.clone();
        endless.diagrams[0].ret = Rich::plain("ketszer(a)");
        let v = run(&endless, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
        assert!(
            v.message.contains("calls itself too deeply"),
            "{}",
            v.message
        );

        let mut silent = p.clone();
        silent.diagrams[0].show_return = false;
        let v = run(&silent, spec, "n: 3\nx: [4, 9, -2]\ns: 22\nh: 2");
        assert!(v.message.contains("has no return value"), "{}", v.message);
    }

    #[test]
    fn arrays_records_loops_and_cases() {
        let spec = "In:\nn∈N\nOut:\ny∈N[0..n],\nm∈N[1..2,1..2],\np∈P,\nP=(a:N x b:N),\nk∈N\nPre:\n-\nPost:\n∀i∈[0..n]:(y[i]=i*i) and m=[[1,2],[3,4]] and p.a=n and p.b=1 and k=3\n";
        let fill = vec![st("m[r,c]:=(r-1)*2+c")];
        let cases = node(Kind::Switch {
            cases: vec![
                Case {
                    id: fresh_id(),
                    cond: Some(Rich::plain("n<0")),
                    body: vec![st("k:=0")],
                },
                Case {
                    id: fresh_id(),
                    cond: None,
                    body: vec![st("k:=0")],
                },
            ],
        });
        let body = vec![
            during("i=0..n", vec![st("y[i]:=i*i")]),
            during("r=1..2", vec![during("c=1..2", fill)]),
            st("p.a:=n"),
            st("p.b:=1"),
            cases,
            node(Kind::Repeat {
                cond: Rich::plain("k<3"),
                body: vec![st("k:=k+1")],
            }),
        ];
        let p = program(vec![("P", "", body)]);
        let v = run(
            &p,
            spec,
            "n: 2\ny: [0, 1, 4]\nm: [[1, 2], [3, 4]]\np: {a: 2, b: 1}\nk: 3",
        );
        assert!(v.pass, "{}", v.message);
        assert_eq!(v.values[0], ("y".into(), "[0, 1, 4]".into()));

        let none = node(Kind::Switch {
            cases: vec![Case {
                id: fresh_id(),
                cond: Some(Rich::plain("n<0")),
                body: vec![],
            }],
        });
        let p = program(vec![("P", "", vec![none])]);
        let v = run(
            &p,
            spec,
            "n: 2\ny: [0, 1, 4]\nm: [[1, 2], [3, 4]]\np: {a: 2, b: 1}\nk: 3",
        );
        assert_eq!(v.node, Some(p.diagrams[0].body[0].id));
        assert_eq!(
            v.message,
            "None of the cases of the multi-way decision holds"
        );
    }

    #[test]
    fn generated_structograms_pass_their_test_cases() {
        let table = "Be:\nn∈Z,\nm∈Z,\nx∈Z[1..n],\nt∈Z[1..n,1..m]\nKi:\n";
        let rest = " és n=n és m=m és x=x és t=t\n";
        let data = "n: 3\nm: 2\nx: [5, -2, 7]\nt: [[0, 0], [1, -4], [2, 3]]\n";
        let cases = [
            (
                "van∈L\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(SZUMMA(j=1..m, t[i,j])>0)",
                "van: true",
            ),
            (
                "van∈L\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(SZUMMA(j=1..m, t[i,j])>9)",
                "van: false",
            ),
            (
                "van∈L\nEf:\n-\nUf:\nvan=∀i∈[1..n]:(DARAB(j=1..m, t[i,j]>0)>0)",
                "van: false",
            ),
            (
                "van∈L\nEf:\n-\nUf:\nvan=∀i∈[1..n]:(DARAB(j=1..m, t[i,j]>9)=0)",
                "van: true",
            ),
            (
                "van∈L,\nk∈N\nEf:\n-\nUf:\n(van,k)=KERES(i=1..n, SZUMMA(j=1..m, t[i,j])>3)",
                "van: true\nk: 3",
            ),
            (
                "van∈L,\nk∈N\nEf:\n-\nUf:\n(van,k)=KERES(i=1..n, SZUMMA(j=1..m, t[i,j])>30)",
                "van: false\nk: 0",
            ),
            (
                "k∈N\nEf:\n-\nUf:\nk=KIVÁLASZT(i>=1, SZUMMA(j=1..m, t[i,j])<0)",
                "k: 2",
            ),
            (
                "mi∈N,\nme∈Z\nEf:\nn>0\nUf:\n(mi,me)=MAX(i=1..n, SZUMMA(j=1..m, t[i,j]))",
                "mi: 3\nme: 5",
            ),
            (
                "mi∈N,\nme∈Z\nEf:\nn>0\nUf:\n(mi,me)=MIN(i=1..n, SZUMMA(j=1..m, t[i,j]))",
                "mi: 2\nme: -3",
            ),
            (
                "s∈Z\nEf:\n-\nUf:\ns=SZUMMA(i=1..n, SZUMMA(i=1..m, t[2,i]))",
                "s: -9",
            ),
            ("s∈Z\nEf:\n-\nUf:\n(n>m -> s=n) és (n≤m -> s=m)", "s: 3"),
            ("s∈Z\nEf:\n-\nUf:\n(n<m -> s=n) és (m≤n -> s=m)", "s: 2"),
            (
                "s∈Z\nEf:\n-\nUf:\n(n>3 -> s=1) és (n=3 -> s=0) és (n<3 -> s=-1)",
                "s: 0",
            ),
            (
                "van∈L,\nmi∈N,\nme∈Z\nEf:\n-\nUf:\n(van,mi,me)=MAXHA(i=1..n, x[i], DARAB(j=1..m, t[i,j]>0)>0)",
                "van: true\nmi: 3\nme: 7",
            ),
            (
                "s∈Z\nEf:\n-\nUf:\n(DARAB(i=1..n, x[i]>0)>1 -> s=1) és (DARAB(i=1..n, x[i]>0)≤1 -> s=0)",
                "s: 1",
            ),
            (
                "db∈N,\ny∈Z[1..db]\nEf:\n-\nUf:\n(db,y)=KIVÁLOGAT(i=1..n, x[i]>0, x[i]*2)",
                "db: 2\ny: [10, 14]",
            ),
            (
                "y∈Z[1..n]\nEf:\n-\nUf:\ny=MÁSOL(i=1..n, SZUMMA(j=1..m, t[i,j]))",
                "y: [0, -3, 5]",
            ),
            // shapes that used to be left as red statements
            (
                "y∈Z[1..n]\nEf:\n-\nUf:\ny[1]=x[1] és ∀i∈[2..n]:(y[i]=y[i-1]+x[i])",
                "y: [5, 3, 10]",
            ),
            (
                "y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:((x[i]>0 -> y[i]=1) és (x[i]≤0 -> y[i]=0))",
                "y: [1, 0, 1]",
            ),
            (
                "y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:(x[i]>0 -> y[i]=x[i]) és ∀i∈[1..n]:(x[i]≤0 -> y[i]=0)",
                "y: [5, 0, 7]",
            ),
            (
                "y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..2]:(y[i]=x[i]) és ∀i∈[3..n]:(y[i]=0)",
                "y: [5, -2, 0]",
            ),
            ("s∈Z\nEf:\n-\nUf:\ns=SZUMMA(i=1..n, x[i]) és s≥0", "s: 10"),
        ];
        for (tail, outputs) in cases {
            let text = format!("{table}{tail}{rest}");
            let spec = parse(&text).unwrap_or_else(|e| panic!("{tail}: {}", e[0].title));
            let test = format!("{data}{outputs}");
            assert!(
                crate::run_test(&spec, &test).pass,
                "{tail}: the test case itself fails"
            );
            let built = from_spec(&spec, "");
            assert!(built.unset.is_empty(), "{tail}");
            let v = run_tests(&built.program, &spec, &[test], &AtomicBool::new(false)).remove(0);
            assert!(v.pass, "{tail}\n{}\n{:?}", v.message, v.values);
        }
    }
}
