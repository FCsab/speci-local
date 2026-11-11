//! Static checks on a parsed specification: undeclared names, unknown types, unused variables.

use super::ast::{BUILTINS, Expr, ExprKind, MACROS, Role, Spec, Type, walk};
use crate::diag::{Diagnostic, did_you_mean};
use std::collections::HashSet;

pub(super) fn check(spec: &Spec) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    unknown_names(spec, &mut diags);
    unknown_types(spec, &mut diags);
    unused_in_post(spec, &mut diags);
    outputs_in_pre(spec, &mut diags);
    diags.sort_by_key(|d| (d.line, d.col));
    diags
}

fn at(spec: &Spec, offset: usize, title: String) -> Diagnostic {
    let (l, c) = spec.pos(offset);
    Diagnostic::new(l, c, title)
}

// ---------------------------------------------------------------- names

enum Unknown<'s> {
    Var(&'s str),
    Func(&'s str),
    Arity {
        name: &'s str,
        want: usize,
        got: usize,
    },
}

impl<'s> Unknown<'s> {
    fn name(&self) -> &'s str {
        match self {
            Unknown::Var(n) | Unknown::Func(n) | Unknown::Arity { name: n, .. } => n,
        }
    }
}

fn unknown_in<'s>(
    spec: &'s Spec,
    e: &'s Expr,
    scope: &mut Vec<&'s str>,
    out: &mut Vec<(usize, Unknown<'s>)>,
) {
    match &*e.kind {
        ExprKind::Var(n) => {
            if !scope.contains(&n.as_str()) && spec.decl(n).is_none() {
                out.push((e.start, Unknown::Var(n)));
            }
        }
        ExprKind::Call(n, args) => {
            let arity = BUILTINS
                .iter()
                .find(|b| b.0 == n)
                .map(|b| b.1)
                .or_else(|| spec.funcs.get(n).map(|f| f.params.len()));
            match arity {
                None => out.push((e.start, Unknown::Func(n))),
                Some(want) if want != args.len() => out.push((
                    e.start,
                    Unknown::Arity {
                        name: n,
                        want,
                        got: args.len(),
                    },
                )),
                Some(_) => {}
            }
            for a in args {
                unknown_in(spec, a, scope, out);
            }
        }
        ExprKind::Quant {
            var, lo, hi, body, ..
        } => {
            unknown_in(spec, lo, scope, out);
            unknown_in(spec, hi, scope, out);
            scope.push(var);
            unknown_in(spec, body, scope, out);
            scope.pop();
        }
        ExprKind::Macro {
            var, lo, hi, args, ..
        } => {
            unknown_in(spec, lo, scope, out);
            if let Some(h) = hi {
                unknown_in(spec, h, scope, out);
            }
            scope.push(var);
            for a in args {
                unknown_in(spec, a, scope, out);
            }
            scope.pop();
        }
        _ => {
            for c in e.children() {
                unknown_in(spec, c, scope, out);
            }
        }
    }
}

fn type_exprs<'t>(t: &'t Type, out: &mut Vec<&'t Expr>) {
    match t {
        Type::Interval(a, b) => out.extend([a, b]),
        Type::Array { lo, hi, elem } => {
            out.push(lo);
            out.extend(hi.iter());
            type_exprs(elem, out);
        }
        Type::Record(fs) => fs.iter().for_each(|(_, t)| type_exprs(t, out)),
        _ => {}
    }
}

// scoped identifier check
fn unknown_names(spec: &Spec, diags: &mut Vec<Diagnostic>) {
    let mut bad = Vec::new();
    for f in spec.funcs.values() {
        let mut scope: Vec<&str> = f.params.iter().map(String::as_str).collect();
        unknown_in(spec, &f.body, &mut scope, &mut bad);
    }
    let mut top_level = vec![&spec.pre, &spec.post];
    top_level.extend(spec.decls.iter().filter_map(|d| d.init.as_ref()));
    for d in &spec.decls {
        type_exprs(&d.ty, &mut top_level);
    }
    for t in spec.aliases.values() {
        type_exprs(t, &mut top_level);
    }
    for e in top_level {
        unknown_in(spec, e, &mut Vec::new(), &mut bad);
    }

    let declared: Vec<&str> = spec.decls.iter().map(|d| d.name.as_str()).collect();
    let mut seen = HashSet::new();
    for (off, unknown) in bad {
        if !seen.insert((off, unknown.name())) {
            continue;
        }
        diags.push(match unknown {
            Unknown::Arity { name, want, got } => at(
                spec,
                off,
                format!("`{name}` takes {want} argument(s) but gets {got} here"),
            ),
            Unknown::Func(name) => unknown_function(spec, off, name),
            Unknown::Var(name) => {
                let d = at(spec, off, format!("`{name}` is not declared"));
                let s = did_you_mean(name, declared.iter().copied());
                let d = d.doc("ch02-01-blocks.html").explain(format!(
                    "Every variable must be declared in an In/Aux/Out (Be/Sa/Ki) block.{s}"
                ));
                if s.is_empty() {
                    d.hint(format!(
                        "Add `{name}∈…` to the right block, or check the spelling."
                    ))
                } else {
                    d
                }
            }
        });
    }
}

fn unknown_function(spec: &Spec, off: usize, name: &str) -> Diagnostic {
    let d = at(spec, off, format!("Unknown function `{name}`"));
    let upper = name.to_uppercase();
    if MACROS.iter().any(|m| m.0 == upper) {
        return d
            .doc("ch06-00-theorems.html")
            .explain("Theorem macros are written in uppercase.")
            .hint(format!("Write `{upper}(…)`."));
    }
    let cands = spec
        .funcs
        .keys()
        .map(String::as_str)
        .chain(BUILTINS.iter().map(|b| b.0));
    d.doc("ch05-00-functions.html").explain(format!(
        "Functions must be defined in a `Fn:`/`Fv:` block. Built-in ones: length (hossz), from (tól), to (ig), abs, sqrt, round (kerekít).{}",
        did_you_mean(name, cands)
    ))
}

// ---------------------------------------------------------------- types

fn named_types<'t>(t: &'t Type, out: &mut Vec<&'t str>) {
    match t {
        Type::Named(n) => out.push(n),
        Type::Array { elem, .. } => named_types(elem, out),
        Type::Record(fs) => fs.iter().for_each(|(_, t)| named_types(t, out)),
        _ => {}
    }
}

// unknown named types
fn unknown_types(spec: &Spec, diags: &mut Vec<Diagnostic>) {
    let mut used = Vec::new();
    for d in &spec.decls {
        named_types(&d.ty, &mut used);
    }
    for t in spec.aliases.values() {
        named_types(t, &mut used);
    }
    let mut seen = HashSet::new();
    for n in used {
        if spec.aliases.contains_key(n) || !seen.insert(n) {
            continue;
        }
        let off = find_word(&spec.src, n).unwrap_or(0);
        diags.push(at(spec, off, format!("Unknown type `{n}`")).doc("ch03-05-aliases.html").explain(format!(
            "Define it in a declaration block, e.g. `{n}=(name:S x value:N)`. Built-in sets: N, Z, R, L, S, C, K.{}",
            did_you_mean(n, spec.aliases.keys().map(String::as_str))
        )));
    }
}

/// Byte offset of `word` as a whole identifier (not inside a longer name).
fn find_word(src: &str, word: &str) -> Option<usize> {
    let is_id = |c: char| c.is_alphanumeric() || c == '_';
    src.match_indices(word).map(|(i, _)| i).find(|&i| {
        !src[..i].chars().next_back().is_some_and(is_id)
            && !src[i + word.len()..].chars().next().is_some_and(is_id)
    })
}

// ---------------------------------------------------------------- use of variables

fn condition_keywords(spec: &Spec) -> (&'static str, &'static str) {
    if spec.post_kw == "Uf:" {
        ("Ef:", "Uf:")
    } else {
        ("Pre:", "Post:")
    }
}

// All Variables Mandate: every input appears in Post (directly or via called functions)
fn unused_in_post(spec: &Spec, diags: &mut Vec<Diagnostic>) {
    let mut in_post: HashSet<&str> = HashSet::new();
    let mut fn_todo: Vec<&str> = Vec::new();
    let mut fn_done: HashSet<&str> = HashSet::new();
    let mut collect = |e, todo: &mut Vec<_>| {
        walk(e, &mut |x| match &*x.kind {
            ExprKind::Var(n) => {
                in_post.insert(n.as_str());
            }
            ExprKind::Call(n, _) => todo.push(n.as_str()),
            _ => {}
        })
    };
    collect(&spec.post, &mut fn_todo);
    while let Some(f) = fn_todo.pop() {
        if fn_done.insert(f)
            && let Some(func) = spec.funcs.get(f)
        {
            collect(&func.body, &mut fn_todo);
        }
    }
    let (prekw, postkw) = condition_keywords(spec);
    for d in &spec.decls {
        if in_post.contains(d.name.as_str()) {
            continue;
        }
        diags.push(
            Diagnostic::new(
                d.line,
                d.col,
                format!(
                    "{} `{}` is never used in the postcondition",
                    d.role.noun(),
                    d.name
                ),
            )
            .doc("ch02-03-all-variables.html").explain(format!(
                "Every declared variable (input, auxiliary and output, constants too) must appear in `{postkw}` (directly or inside a function it calls). Using it only in `{prekw}` doesn't count. The online editor reports this as \"{} is missing from the post condition!\".",
                d.name
            ))
            .hint(format!(
                "If it really isn't needed, append `and {0}={0}` to the postcondition.",
                d.name
            )),
        );
    }
}

// outputs belong to the postcondition
fn outputs_in_pre(spec: &Spec, diags: &mut Vec<Diagnostic>) {
    let (_, postkw) = condition_keywords(spec);
    let mut seen = HashSet::new();
    walk(&spec.pre, &mut |x| {
        if let ExprKind::Var(n) = &*x.kind
            && spec.decl(n).is_some_and(|d| d.role == Role::Out)
            && seen.insert(n.as_str())
        {
            diags.push(
                at(spec, x.start, format!("Output `{n}` is used in the precondition"))
                    .doc("ch04-03-preconditions.html").explain(format!("The precondition may only talk about the input. The online editor reports this as \"Output data {n} is in the precondition!\"."))
                    .hint(format!("Move the condition on `{n}` into `{postkw}`.")),
            );
        }
    });
}
