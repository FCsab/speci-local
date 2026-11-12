//! The parsed specification: expressions, types, declarations and the names the language knows.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: Box<ExprKind>,
    pub start: usize, // byte offsets into Spec::src
    pub end: usize,
    pub depth: u32,
}

pub const MAX_DEPTH: u32 = 400;

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Real(f64),
    Str(String),
    Bool(bool),
    Var(String),
    Tuple(Vec<Option<Expr>>),
    /// Array literal; `from` holds the start indexes of `[0..:1, 2]` / `[1..,1..:[1,2],[3,4]]`.
    Array(Vec<Expr>, Vec<Expr>),
    Neg(Expr),
    Not(Expr),
    Bin(BinOp, Expr, Expr),
    /// Chained comparison: `a < b <= c` → ops.len() == operands.len() - 1
    Cmp(Vec<Expr>, Vec<&'static str>),
    /// `x ∈ [lo..hi]`
    In(Expr, Expr, Expr),
    Index(Expr, Vec<Expr>),
    Field(Expr, String),
    Call(String, Vec<Expr>),
    Quant {
        all: bool,
        var: String,
        lo: Expr,
        hi: Expr,
        body: Expr,
    },
    Macro {
        name: String,
        var: String,
        lo: Expr,
        hi: Option<Expr>,
        args: Vec<Expr>,
    },
    /// `{a, ha c; b egyébként}`: the value of the first case whose condition holds, as
    /// (value, condition) pairs, then the `egyébként` (otherwise) value.
    Cases(Vec<(Expr, Expr)>, Option<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    /// `div`: whole-number division rounded down
    IDiv,
    And,
    Or,
    Imp,
}

#[derive(Clone, Debug)]
pub enum Type {
    Nat,
    Int,
    Real,
    Bool,
    Str,
    Char,
    Interval(Expr, Expr),
    Array {
        lo: Expr,
        hi: Option<Expr>,
        elem: Box<Type>,
    },
    Record(Vec<(String, Type)>),
    Named(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    In,
    Aux,
    Out,
}

impl Role {
    pub fn noun(self) -> &'static str {
        match self {
            Role::In => "Input",
            Role::Aux => "Auxiliary variable",
            Role::Out => "Output",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Decl {
    pub name: String,
    pub role: Role,
    pub ty: Type,
    pub init: Option<Expr>,
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub params: Vec<String>,
    /// The signature's types: domain (one per parameter, when written that way) and codomain.
    pub dom: Vec<Type>,
    pub cod: Vec<Type>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub src: String,
    pub decls: Vec<Decl>,
    pub aliases: HashMap<String, Type>,
    pub funcs: HashMap<String, Func>,
    pub pre: Expr,
    pub post: Expr,
    /// Header names as written, for messages (`Uf:` vs `Post:`).
    pub pre_kw: String,
    pub post_kw: String,
}

impl Spec {
    pub fn text(&self, e: &Expr) -> &str {
        self.src.get(e.start..e.end).unwrap_or("?").trim()
    }
    pub fn decl(&self, name: &str) -> Option<&Decl> {
        self.decls.iter().find(|d| d.name == name)
    }
    pub fn pos(&self, offset: usize) -> (usize, usize) {
        line_col(&self.src, offset)
    }
    /// Follow alias chains to a structural type.
    pub fn resolve<'a>(&'a self, mut t: &'a Type) -> Option<&'a Type> {
        for _ in 0..50 {
            match t {
                Type::Named(n) => t = self.aliases.get(n)?,
                _ => return Some(t),
            }
        }
        None
    }
}

pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(src.len());
    let before = &src[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    (line, before[line_start..].chars().count() + 1)
}

pub const MACROS: &[(&str, usize, usize, &str)] = &[
    // name, min args, max args (after the range), usage
    (
        "SUM",
        1,
        2,
        "SUM(i=b..e, f(i)) or SUM(i=b..e, f(i), condition)",
    ),
    ("COUNT", 1, 1, "COUNT(i=b..e, condition)"),
    ("DARAB", 1, 1, "DARAB(i=b..e, condition)"),
    ("MAX", 1, 1, "(ind, value)=MAX(i=b..e, f(i))"),
    ("MIN", 1, 1, "(ind, value)=MIN(i=b..e, f(i))"),
    ("EXISTS", 1, 1, "EXISTS(i=b..e, condition)"),
    ("FORALL", 1, 1, "FORALL(i=b..e, condition)"),
    ("SELECT", 1, 1, "ind=SELECT(i>=b, condition)"),
    ("SEARCH", 1, 1, "(van, ind)=SEARCH(i=b..e, condition)"),
    ("COPY", 1, 1, "y=COPY(i=b..e, f(i))"),
    (
        "MIS",
        1,
        2,
        "(cnt, y)=MIS(i=b..e, condition) or MIS(i=b..e, condition, f(i))",
    ),
    (
        "CONDMAX",
        2,
        2,
        "(van, ind, value)=CONDMAX(i=b..e, f(i), condition)",
    ),
    (
        "CONDMIN",
        2,
        2,
        "(van, ind, value)=CONDMIN(i=b..e, f(i), condition)",
    ),
];

/// Other names the online editor accepts for the patterns → the name used here.
pub fn macro_alias(name: &str) -> Option<&'static str> {
    Some(match name {
        "SZUMMA" | "SZUM" => "SUM",
        "MEGSZÁMOL" => "COUNT",
        "ELDÖNT" | "DECISION" | "VAN" | "LÉTEZIK" => "EXISTS",
        "MIND" | "EVERY" => "FORALL",
        "KERES" => "SEARCH",
        "KIVÁLASZT" => "SELECT",
        "MÁSOL" | "MAP" => "COPY",
        "KIVÁLOGAT" | "FILTER" => "MIS",
        "MAXHA" | "MAXIF" | "FELTMAX" => "CONDMAX",
        "MINHA" | "MINIF" | "FELTMIN" => "CONDMIN",
        _ => return None,
    })
}

pub const BUILTINS: &[(&str, usize)] = &[
    ("length", 1),
    ("hossz", 1),
    ("from", 1),
    ("tól", 1),
    ("to", 1),
    ("ig", 1),
    ("abs", 1),
    ("sqrt", 1),
    ("round", 1),
    ("kerekít", 1),
    ("!", 1), // postfix factorial `n!`
];

impl Expr {
    pub fn children(&self) -> Vec<&Expr> {
        match &*self.kind {
            ExprKind::Tuple(s) => s.iter().flatten().collect(),
            ExprKind::Array(v, f) => v.iter().chain(f).collect(),
            ExprKind::Call(_, v) | ExprKind::Cmp(v, _) => v.iter().collect(),
            ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Field(a, _) => vec![a],
            ExprKind::Bin(_, a, b) => vec![a, b],
            ExprKind::In(a, b, c) => vec![a, b, c],
            ExprKind::Index(a, v) => std::iter::once(a).chain(v.iter()).collect(),
            ExprKind::Quant { lo, hi, body, .. } => vec![lo, hi, body],
            ExprKind::Macro { lo, hi, args, .. } => std::iter::once(lo)
                .chain(hi.iter())
                .chain(args.iter())
                .collect(),
            ExprKind::Cases(arms, otherwise) => arms
                .iter()
                .flat_map(|(v, c)| [v, c])
                .chain(otherwise)
                .collect(),
            ExprKind::Int(_)
            | ExprKind::Real(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Var(_) => vec![],
        }
    }
}

pub(super) fn walk<'e>(e: &'e Expr, f: &mut dyn FnMut(&'e Expr)) {
    f(e);
    for c in e.children() {
        walk(c, f);
    }
}

/// Variable names mentioned anywhere in an expression (for explaining failures).
pub fn vars_in(e: &Expr) -> Vec<String> {
    let mut v = Vec::new();
    walk(e, &mut |x| {
        if let ExprKind::Var(n) = &*x.kind
            && !v.contains(n)
        {
            v.push(n.clone());
        }
    });
    v
}
