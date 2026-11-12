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

mod compile;
mod machine;
#[cfg(test)]
mod tests;

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
