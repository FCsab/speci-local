//! Running a test case against a specification, and explaining the verdict.

use crate::data;
use crate::diag::Diagnostic;
use crate::eval::{Env, Eval, Trace, Value};
use crate::parser::{BinOp, Decl, Expr, ExprKind, Role, Spec, Type, vars_in};
use std::fmt::Write;
use std::sync::atomic::AtomicBool;

/// Result of running one test case.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub pass: bool,
    pub diags: Vec<Diagnostic>,
    /// Aux/Out variables after evaluation: (name, value, taken from the test data?).
    pub values: Vec<(String, String, bool)>,
}

impl Outcome {
    fn failed(d: Diagnostic) -> Outcome {
        Outcome {
            pass: false,
            diags: vec![d],
            values: vec![],
        }
    }
}

fn conjuncts(e: &Expr) -> Vec<&Expr> {
    match &*e.kind {
        ExprKind::Bin(BinOp::And, a, b) => {
            let mut v = conjuncts(a);
            v.extend(conjuncts(b));
            v
        }
        _ => vec![e],
    }
}

fn values_of(e: &Expr, env: &Env) -> String {
    vars_in(e)
        .iter()
        .filter_map(|n| env.get(n).map(|v| format!("{n} = {}", v.short())))
        .collect::<Vec<_>>()
        .join(", ")
}

fn explain_quantifier(ev: &mut Eval, part: &Expr, env: &mut Env) -> Option<String> {
    let ExprKind::Quant {
        all,
        var,
        lo,
        hi,
        body,
    } = &*part.kind
    else {
        return None;
    };
    let bounds = (ev.eval_scratch(lo, env), ev.eval_scratch(hi, env));
    let (Ok(Value::Int(a)), Ok(Value::Int(b))) = bounds else {
        return None;
    };
    if !*all {
        return Some(if a > b {
            format!("The range {a}..{b} is empty, so ∃ is false.")
        } else {
            format!("No {var} in {a}..{b} makes it true.")
        });
    }
    let shadowed = env.remove(var);
    let mut why = None;
    for i in a..=b {
        env.insert(var.clone(), Value::Int(i));
        if let Ok(false) = ev.truth_scratch(body, env) {
            let mut at_failure = env.clone();
            let _ = ev.truth(body, &mut at_failure);
            why = Some(format!(
                "It fails for {var} = {i}: {}.",
                values_of(body, &at_failure)
            ));
            break;
        }
    }
    match shadowed {
        Some(v) => env.insert(var.clone(), v),
        None => env.remove(var),
    };
    why
}

/// Find the first top-level `and` part that is false and say why.
pub(crate) fn explain_false(ev: &mut Eval, e: &Expr, env: &Env, which: &str) -> Diagnostic {
    let spec = ev.spec;
    let mut c = env.clone();
    for part in conjuncts(e) {
        match ev.truth(part, &mut c) {
            Err(d) => return d,
            Ok(true) => continue,
            Ok(false) => {}
        }
        let (l, col) = spec.pos(part.start);
        let d = Diagnostic::new(
            l,
            col,
            format!("The {which} is false: `{}` does not hold", spec.text(part)),
        );
        let why = explain_quantifier(ev, part, &mut c).unwrap_or_else(|| {
            let vals = values_of(part, &c);
            if vals.is_empty() {
                String::new()
            } else {
                format!("Values used: {vals}.")
            }
        });
        return d.explain(why);
    }
    let (l, col) = spec.pos(e.start);
    Diagnostic::new(l, col, format!("The {which} is false"))
}

/// Run one test case against a parsed specification.
pub fn run_test(spec: &Spec, data: &str) -> Outcome {
    run_test_with(spec, data, &AtomicBool::new(false))
}

/// Record every evaluation step of one test case (precondition, then postcondition) for the stepper.
/// Returns `None` if the test data can't be loaded (the normal run reports why).
pub fn trace_test(spec: &Spec, data: &str, cancel: &AtomicBool) -> Option<Trace> {
    let env = data::load(spec, data).ok()?;
    Some(trace_env(spec, &env, cancel))
}

pub fn run_and_trace(spec: &Spec, data: &str, cancel: &AtomicBool) -> (Outcome, Option<Trace>) {
    match data::load(spec, data) {
        Ok(env) => {
            let trace = trace_env(spec, &env, cancel);
            (run_env(spec, env, cancel), Some(trace))
        }
        Err(d) => (Outcome::failed(d), None),
    }
}

fn trace_env(spec: &Spec, env: &Env, cancel: &AtomicBool) -> Trace {
    let mut ev = Eval::with_cancel(spec, cancel);
    ev.trace = Some(Trace::default());
    ev.set_root(&spec.pre, &spec.pre_kw);
    if let Ok(true) = ev.truth(&spec.pre, &mut env.clone()) {
        ev.set_root(&spec.post, &spec.post_kw);
        let _ = ev.truth(&spec.post, &mut env.clone());
    }
    ev.trace.unwrap_or_default()
}

fn shown_values(outs: &[&Decl], env: &Env, from_test: bool) -> Vec<(String, String, bool)> {
    outs.iter()
        .map(|d| {
            let value = env
                .get(&d.name)
                .map_or_else(|| "— (no value)".into(), Value::summary);
            (d.name.clone(), value, from_test)
        })
        .collect()
}

/// Like [`run_test`], but gives up (failing the test) as soon as `cancel` is set.
pub fn run_test_with(spec: &Spec, data: &str, cancel: &AtomicBool) -> Outcome {
    match data::load(spec, data) {
        Ok(env) => run_env(spec, env, cancel),
        Err(d) => Outcome::failed(d),
    }
}

fn run_env(spec: &Spec, env: Env, cancel: &AtomicBool) -> Outcome {
    let mut ev = Eval::with_cancel(spec, cancel);

    match ev.truth(&spec.pre, &mut env.clone()) {
        Err(d) => return Outcome::failed(d),
        Ok(false) => {
            let which = format!("precondition ({})", spec.pre_kw.trim_end_matches(':'));
            let d = explain_false(&mut ev, &spec.pre, &env, &which);
            return Outcome::failed(d.hint("This test case's input is not allowed by the precondition. Either the test data or the precondition is wrong."));
        }
        Ok(true) => {}
    }

    let outs: Vec<&Decl> = spec.decls.iter().filter(|d| d.role != Role::In).collect();
    let mut post_env = env.clone();
    match ev.truth(&spec.post, &mut post_env) {
        Err(d) => Outcome {
            pass: false,
            diags: vec![d],
            values: shown_values(&outs, &post_env, true),
        },
        Ok(true) => Outcome {
            pass: true,
            diags: vec![],
            values: shown_values(&outs, &post_env, true),
        },
        Ok(false) => post_failed(&mut ev, &env, &outs),
    }
}

fn post_failed(ev: &mut Eval, env: &Env, outs: &[&Decl]) -> Outcome {
    let spec = ev.spec;
    let which = format!("postcondition ({})", spec.post_kw.trim_end_matches(':'));
    let mut d = explain_false(ev, &spec.post, env, &which);
    // what would the specification produce without the expected outputs?
    let mut free = env.clone();
    for o in outs {
        free.remove(&o.name);
    }
    let mut diffs = Vec::new();
    let mut computed = None;
    match ev.truth(&spec.post, &mut free) {
        Ok(true) => {
            for o in outs {
                if let (Some(want), Some(got)) = (env.get(&o.name), free.get(&o.name))
                    && !want.eq_val(got)
                {
                    diffs.push(format!(
                        "`{}`: the test case says {}, the specification gives {}",
                        o.name,
                        want.short(),
                        got.short()
                    ));
                }
            }
            computed = Some(free);
        }
        Ok(false) => diffs.push("Even with the expected outputs removed, no output values satisfy the postcondition for this input. The formula itself is probably wrong.".into()),
        Err(_) => {}
    }
    if !diffs.is_empty() {
        let sep = if d.explanation.is_empty() { "" } else { "\n" };
        d.explanation = format!("{}{sep}{}", d.explanation, diffs.join("\n"));
    }
    d = d.hint("Either the expected output in the test case is wrong, or the postcondition does not describe the task.");
    Outcome {
        pass: false,
        diags: vec![d],
        values: match &computed {
            Some(c) => shown_values(outs, c, false),
            None => shown_values(outs, env, true),
        },
    }
}

/// Lines to scaffold into a test case: `name: ` for each variable not already present.
pub fn scaffold(spec: &Spec, existing: &str) -> String {
    let mut out = String::new();
    for d in &spec.decls {
        let present = existing.lines().any(|l| {
            l.trim_start()
                .strip_prefix(d.name.as_str())
                .is_some_and(|r| r.trim_start().starts_with(':'))
        });
        if present || (d.init.is_some() && d.role == Role::In) {
            continue;
        }
        let value = match spec.resolve(&d.ty) {
            Some(Type::Array { elem, .. })
                if matches!(spec.resolve(elem), Some(Type::Array { .. })) =>
            {
                "[[]]".to_string()
            }
            Some(Type::Array { .. }) => "[]".to_string(),
            Some(Type::Record(fs)) => {
                let fields: Vec<String> = fs.iter().map(|(n, _)| format!("{n}: ")).collect();
                format!("{{{}}}", fields.join(", "))
            }
            _ => String::new(),
        };
        let _ = writeln!(out, "{}: {value}", d.name);
    }
    out
}
