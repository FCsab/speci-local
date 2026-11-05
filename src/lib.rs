pub mod data;
pub mod diag;
pub mod eval;
pub mod file;
pub mod parser;

use diag::Diagnostic;
use eval::{Env, Eval, Value};
use parser::{BinOp, Expr, ExprKind, Role, Spec, vars_in};

pub use parser::parse;

/// Result of running one test case.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub pass: bool,
    pub diags: Vec<Diagnostic>,
    /// Aux/Out variables after evaluation: (name, value, came from the test data?)
    pub values: Vec<(String, String, bool)>,
    pub notes: Vec<String>,
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

/// Find the first top-level `and` part that is false and say why.
fn explain_false(ev: &mut Eval, e: &Expr, env: &Env, which: &str) -> Diagnostic {
    let spec = ev.spec;
    let mut c = env.clone();
    for part in conjuncts(e) {
        match ev.truth(part, &mut c) {
            Err(d) => return d,
            Ok(true) => continue,
            Ok(false) => {}
        }
        let (l, col) = spec.pos(part.start);
        let mut d = Diagnostic::new(
            l,
            col,
            format!("The {which} is false: `{}` does not hold", spec.text(part)),
        );
        let mut why = Vec::new();
        if let ExprKind::Quant {
            all,
            var,
            lo,
            hi,
            body,
        } = &*part.kind
        {
            let bounds = (ev.eval(lo, &mut c.clone()), ev.eval(hi, &mut c.clone()));
            if let (Ok(Value::Int(a)), Ok(Value::Int(b))) = bounds {
                if *all {
                    for i in a..=b {
                        let mut ci = c.clone();
                        ci.insert(var.clone(), Value::Int(i));
                        if let Ok(false) = ev.truth(body, &mut ci) {
                            why.push(format!(
                                "It fails for {var} = {i}: {}.",
                                values_of(body, &ci)
                            ));
                            break;
                        }
                    }
                } else if a > b {
                    why.push(format!("The range {a}..{b} is empty, so ∃ is false."));
                } else {
                    why.push(format!("No {var} in {a}..{b} makes it true."));
                }
            }
        }
        let vals = values_of(part, &c);
        if !vals.is_empty() && why.is_empty() {
            why.push(format!("Values used: {vals}."));
        }
        d = d.explain(why.join(" "));
        return d;
    }
    let (l, col) = spec.pos(e.start);
    Diagnostic::new(l, col, format!("The {which} is false"))
}

/// Run one test case against a parsed specification.
pub fn run_test(spec: &Spec, data: &str) -> Outcome {
    run_test_with(spec, data, &std::sync::atomic::AtomicBool::new(false))
}

/// Does `v` belong to the declared set `t`? Returns a reason if not.
fn misfit(spec: &Spec, t: &parser::Type, v: &Value) -> Option<String> {
    use parser::Type as T;
    let t = spec.resolve(t)?;
    match (t, v) {
        (_, Value::Undef) => None,
        (T::Nat, Value::Int(i)) if *i < 0 => Some(format!(
            "{i} is negative, but N only has whole numbers ≥ 0 (declare it as Z?)"
        )),
        (T::Nat | T::Int, Value::Int(_))
        | (T::Real, Value::Int(_) | Value::Real(_))
        | (T::Bool, Value::Bool(_)) => None,
        (T::Str, Value::Str(_)) => None,
        (T::Char, Value::Str(s)) if s.chars().count() == 1 => None,
        (T::Interval(..), Value::Int(_)) => None,
        (T::Array { elem, .. }, Value::Arr { items, .. }) => {
            items.iter().find_map(|x| misfit(spec, elem, x))
        }
        (T::Record(fs), Value::Rec(vs)) => fs
            .iter()
            .zip(vs)
            .find_map(|((_, ft), (_, fv))| misfit(spec, ft, fv)),
        (_, v) => Some(format!(
            "{} is {}, which doesn't fit the declared type",
            v.short(),
            v.kind()
        )),
    }
}

/// Like [`run_test`], but gives up (failing the test) as soon as `cancel` is set.
pub fn run_test_with(spec: &Spec, data: &str, cancel: &std::sync::atomic::AtomicBool) -> Outcome {
    let fail = |d: Diagnostic| Outcome {
        pass: false,
        diags: vec![d],
        values: vec![],
        notes: vec![],
    };
    let env = match data::load(spec, data) {
        Ok(e) => e,
        Err(d) => return fail(d),
    };
    let mut ev = Eval::with_cancel(spec, cancel);
    let (pre_kw, post_kw) = (
        spec.pre_kw.trim_end_matches(':'),
        spec.post_kw.trim_end_matches(':'),
    );

    match ev.truth(&spec.pre, &mut env.clone()) {
        Err(d) => return fail(d),
        Ok(false) => {
            let d = explain_false(
                &mut ev,
                &spec.pre,
                &env,
                &format!("precondition ({pre_kw})"),
            );
            return fail(d.hint("This test case's input is not allowed by the precondition. Either the test data or the precondition is wrong."));
        }
        Ok(true) => {}
    }

    let outs: Vec<&parser::Decl> = spec.decls.iter().filter(|d| d.role != Role::In).collect();
    let mut post_env = env.clone();
    let result = ev.truth(&spec.post, &mut post_env);
    let values = |e: &Env| {
        outs.iter()
            .map(|d| {
                (
                    d.name.clone(),
                    e.get(&d.name)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "— (no value)".into()),
                    env.contains_key(&d.name),
                )
            })
            .collect::<Vec<_>>()
    };
    match result {
        Err(d) => Outcome {
            pass: false,
            diags: vec![d],
            values: values(&post_env),
            notes: vec![],
        },
        Ok(true) => {
            let notes = outs
                .iter()
                .filter(|d| d.role == Role::Out && !post_env.contains_key(&d.name))
                .map(|d| format!("`{}` got no value: it isn't in the test data and the postcondition doesn't determine it here (e.g. it is behind an implication whose condition is false).", d.name))
                .chain(outs.iter().filter(|d| !env.contains_key(&d.name)).filter_map(|d| {
                    let why = misfit(spec, &d.ty, post_env.get(&d.name)?)?;
                    Some(format!("The computed value of `{}` is not valid for its declaration: {why}.", d.name))
                }))
                .collect();
            Outcome {
                pass: true,
                diags: vec![],
                values: values(&post_env),
                notes,
            }
        }
        Ok(false) => {
            let mut d = explain_false(
                &mut ev,
                &spec.post,
                &env,
                &format!("postcondition ({post_kw})"),
            );
            // what would the specification produce without the expected outputs?
            let mut free = env.clone();
            for o in &outs {
                free.remove(&o.name);
            }
            let mut diffs = Vec::new();
            let mut computed = None;
            match ev.truth(&spec.post, &mut free) {
                Ok(true) => {
                    for o in &outs {
                        if let (Some(want), Some(got)) = (env.get(&o.name), free.get(&o.name))
                            && !want.eq_val(got) {
                                diffs.push(format!("`{}`: the test case says {}, the specification gives {}", o.name, want.short(), got.short()));
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
                values: values(computed.as_ref().unwrap_or(&post_env)),
                notes: vec![],
            }
        }
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
        let tmpl = match spec.resolve(&d.ty) {
            Some(parser::Type::Array { elem, .. })
                if matches!(spec.resolve(elem), Some(parser::Type::Array { .. })) =>
            {
                "[[]]"
            }
            Some(parser::Type::Array { .. }) => "[]",
            Some(parser::Type::Record(fs)) => {
                out.push_str(&format!(
                    "{}: {{{}}}\n",
                    d.name,
                    fs.iter()
                        .map(|(n, _)| format!("{n}: "))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                continue;
            }
            _ => "",
        };
        out.push_str(&format!("{}: {tmpl}\n", d.name));
    }
    out
}
