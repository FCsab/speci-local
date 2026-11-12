//! Small helpers of Auto Stuki: building nodes and reading the expressions.

use super::*;

/// The pattern's name as the evaluator knows it (`MEGSZÁMOL` → `COUNT`).
pub(super) fn canonical(name: &str) -> &str {
    match name {
        "DARAB" => "COUNT",
        _ => macro_alias(name).unwrap_or(name),
    }
}

/// Patterns that give a single value (the others give a tuple or an array).
pub(super) fn single_valued(name: &str) -> bool {
    matches!(
        canonical(name),
        "SUM" | "COUNT" | "EXISTS" | "FORALL" | "SELECT"
    )
}

pub(super) fn node(kind: Kind) -> Node {
    Node {
        id: fresh_id(),
        bg: None,
        kind,
    }
}

pub(super) fn stmt(text: impl AsRef<str>) -> Node {
    node(Kind::Statement(Rich::plain(text.as_ref())))
}

pub(super) fn seq(nodes: Vec<Node>) -> Vec<Node> {
    if nodes.is_empty() {
        vec![Node::noop()]
    } else {
        nodes
    }
}

pub(super) fn if_then(cond: impl AsRef<str>, yes: Vec<Node>) -> Node {
    node(Kind::If {
        cond: Rich::plain(cond.as_ref()),
        yes: seq(yes),
        no: vec![Node::noop()],
    })
}

pub(super) fn while_loop(cond: impl AsRef<str>, body: Vec<Node>) -> Node {
    node(Kind::While {
        cond: Rich::plain(cond.as_ref()),
        body: seq(body),
    })
}

pub(super) fn conjuncts(e: &Expr) -> Vec<&Expr> {
    match &*e.kind {
        ExprKind::Bin(BinOp::And, a, b) => {
            let mut v = conjuncts(a);
            v.extend(conjuncts(b));
            v
        }
        _ => vec![e],
    }
}

/// Needs no parentheses after `nem` or inside a longer expression.
pub(super) fn atomic(e: &Expr) -> bool {
    matches!(
        &*e.kind,
        ExprKind::Var(_)
            | ExprKind::Int(_)
            | ExprKind::Real(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Call(..)
            | ExprKind::Index(..)
            | ExprKind::Field(..)
            | ExprKind::Tuple(_)
            | ExprKind::Array(..)
            | ExprKind::Macro { .. }
    )
}

/// Parentheses never close more than they opened (so `(a)+(b)` keeps its outer ones).
pub(super) fn balanced(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// The loop variables of the patterns and quantifiers in `e`.
pub(super) fn loop_vars(e: &Expr, out: &mut HashSet<String>) {
    if let ExprKind::Quant { var, .. } | ExprKind::Macro { var, .. } = &*e.kind {
        out.insert(var.clone());
    }
    e.children().into_iter().for_each(|c| loop_vars(c, out));
}

/// Line breaks (and the indentation after them) become single spaces.
pub(super) fn one_line(s: &str) -> String {
    let mut out = String::new();
    let mut lines = s.lines().map(str::trim).filter(|l| !l.is_empty());
    if let Some(first) = lines.next() {
        out.push_str(first);
    }
    for l in lines {
        out.push(' ');
        out.push_str(l);
    }
    out
}

/// Has an operator outside brackets: `a+1` needs parentheses when put into `2*…`, `f(a)` doesn't.
pub(super) fn needs_parens(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ if depth == 0 && !(c.is_alphanumeric() || c == '_' || c == '.') => return true,
            _ => {}
        }
    }
    false
}

/// `MAX(…).2` / `MAX(…).maxval`: a pattern with several results, and the position of the one used.
pub(super) fn projection(e: &Expr) -> Option<(&Expr, usize)> {
    let ExprKind::Field(m, f) = &*e.kind else {
        return None;
    };
    let ExprKind::Macro { name, .. } = &*m.kind else {
        return None;
    };
    let names = result_names(canonical(name));
    let k = f
        .parse::<usize>()
        .ok()
        .and_then(|i| i.checked_sub(1))
        .or_else(|| names.iter().position(|n| n == f))?;
    (k < names.len()).then_some((m, k))
}

/// A part of an expression whose value is computed by steps of its own.
pub(super) fn lifted(e: &Expr) -> bool {
    match &*e.kind {
        ExprKind::Quant { .. } => true,
        ExprKind::Macro { name, .. } => single_valued(name),
        _ => projection(e).is_some(),
    }
}

pub(super) fn contains_lift(e: &Expr) -> bool {
    lifted(e) || e.children().into_iter().any(contains_lift)
}

pub(super) fn stray(e: &Expr, statement: bool) -> bool {
    let side = |x: &Expr| match &*x.kind {
        ExprKind::Macro { .. } => x.children().into_iter().any(|c| stray(c, false)),
        _ => stray(x, false),
    };
    match &*e.kind {
        ExprKind::Bin(BinOp::Imp, a, b) if statement => conjuncts(a)
            .into_iter()
            .chain(conjuncts(b))
            .any(|p| stray(p, true)),
        ExprKind::Quant {
            all: true,
            lo,
            hi,
            body,
            ..
        } if statement => {
            stray(lo, false)
                || stray(hi, false)
                || conjuncts(body).into_iter().any(|p| stray(p, true))
        }
        ExprKind::Macro { name, .. } if !single_valued(name) => true,
        _ if let Some((m, _)) = projection(e) => m.children().into_iter().any(|c| stray(c, false)),
        _ => match equation(e) {
            Some((l, r)) if statement => side(l) || side(r),
            _ => e.children().into_iter().any(|c| stray(c, false)),
        },
    }
}

pub(super) fn renamed(
    e: &Expr,
    renames: &[(String, String)],
    hidden: &mut Vec<String>,
    out: &mut Vec<(usize, usize, String)>,
) {
    let mut inside = |var: &String, parts: &[&Expr], hidden: &mut Vec<String>| {
        hidden.push(var.clone());
        for p in parts {
            renamed(p, renames, hidden, out);
        }
        hidden.pop();
    };
    match &*e.kind {
        ExprKind::Var(n) if !hidden.contains(n) => {
            if let Some((_, with)) = renames.iter().rev().find(|(old, _)| old == n) {
                out.push((e.start, e.end, with.clone()));
            }
        }
        ExprKind::Quant {
            var, lo, hi, body, ..
        } => {
            inside(var, &[body], hidden);
            renamed(lo, renames, hidden, out);
            renamed(hi, renames, hidden, out);
        }
        ExprKind::Macro {
            var, lo, hi, args, ..
        } => {
            inside(var, &args.iter().collect::<Vec<_>>(), hidden);
            renamed(lo, renames, hidden, out);
            if let Some(hi) = hi {
                renamed(hi, renames, hidden, out);
            }
        }
        _ => e
            .children()
            .into_iter()
            .for_each(|c| renamed(c, renames, hidden, out)),
    }
}

const NEGATED: [(&str, &str); 4] = [("<", ">="), (">", "<="), ("=", "!="), ("|", "∤")];

pub(super) fn negated(op: &str) -> Option<&'static str> {
    NEGATED.iter().find_map(|&(a, b)| match op {
        _ if op == a => Some(b),
        _ if op == b => Some(a),
        _ => None,
    })
}

pub(super) fn mirrored(op: &str) -> Option<&'static str> {
    Some(match op {
        "<" => ">",
        ">" => "<",
        "<=" => ">=",
        ">=" => "<=",
        "=" => "=",
        "!=" => "!=",
        _ => return None,
    })
}

/// `=` with a target on one side: (target, value).
pub(super) fn equation(e: &Expr) -> Option<(&Expr, &Expr)> {
    match &*e.kind {
        ExprKind::Cmp(v, ops) if v.len() == 2 && ops[0] == "=" => Some((&v[0], &v[1])),
        _ => None,
    }
}
