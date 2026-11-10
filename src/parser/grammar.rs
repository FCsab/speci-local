//! Recursive-descent parser: the token helpers, declarations, types and functions.
//! (Conditions and expressions are in [`super::expr`].)

use super::ast::{Decl, Expr, ExprKind, Func, MAX_DEPTH, Role, Type, line_col};
use super::lexer::{Tok, Token};
use crate::diag::{Diagnostic, did_you_mean};
use std::cell::Cell;
use std::collections::HashMap;

pub(super) struct Parser<'a> {
    pub(super) src: &'a str,
    pub(super) toks: Vec<Token>,
    pub(super) pos: usize,
    pub(super) block_kw: &'a str,
    nesting: usize,
    too_deep: Cell<Option<usize>>,
}

pub(super) const MAX_NESTING: usize = 64;

pub(super) type PResult<T> = Result<T, Diagnostic>;

fn describe(t: &Token) -> String {
    match &t.tok {
        Tok::Eof => "the end of the block".into(),
        Tok::Str(_) => format!("the text {}", t.text),
        _ => format!("`{}`", t.text),
    }
}

fn reserved_meaning(word: &str) -> Option<&'static str> {
    Some(match word {
        "exists" | "létezik" => "the word form of ∃",
        "forall" | "minden" => "the word form of ∀",
        "in" | "eleme" => "the word form of ∈",
        "div" => "whole-number division",
        "igaz" | "hamis" => "a logical constant",
        "and" | "és" => "logical AND",
        "or" | "vagy" => "logical OR",
        "not" | "nem" => "logical NOT",
        "mod" => "the remainder operator",
        "true" | "false" => "a logical constant",
        _ => return None,
    })
}

pub(super) fn bool_word(word: &str) -> Option<bool> {
    match word {
        "true" | "igaz" => Some(true),
        "false" | "hamis" => Some(false),
        _ => None,
    }
}

impl<'a> Parser<'a> {
    pub(super) fn new(src: &'a str, toks: Vec<Token>, block_kw: &'a str) -> Self {
        Parser {
            src,
            toks,
            pos: 0,
            block_kw,
            nesting: 0,
            too_deep: Cell::new(None),
        }
    }

    pub(super) fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        if self.nesting >= MAX_NESTING {
            return Err(self.nesting_error(self.peek().start));
        }
        self.nesting += 1;
        let r = f(self);
        self.nesting -= 1;
        r
    }

    fn nesting_error(&self, offset: usize) -> Diagnostic {
        self.err_at(offset, "The formula is nested too deeply")
            .explain(format!(
                "Brackets, signs, `not`s and quantifiers may be nested {MAX_NESTING} levels deep, and a chain of operators may have a few hundred parts."
            ))
            .hint("Split the formula: move a part of it into a function in the `Fn:` block.")
    }

    pub(super) fn depth_error(&self) -> Option<Diagnostic> {
        self.too_deep.get().map(|at| self.nesting_error(at))
    }

    pub(super) fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }
    pub(super) fn next(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    pub(super) fn is(&self, s: &str) -> bool {
        matches!(&self.peek().tok, Tok::Sym(x) if *x == s)
    }
    pub(super) fn eat(&mut self, s: &str) -> bool {
        if self.is(s) {
            self.next();
            true
        } else {
            false
        }
    }
    pub(super) fn last_end(&self) -> usize {
        if self.pos == 0 {
            self.toks[0].start
        } else {
            self.toks[self.pos - 1].end
        }
    }
    pub(super) fn err_at(&self, offset: usize, title: impl Into<String>) -> Diagnostic {
        let (l, c) = line_col(self.src, offset);
        Diagnostic::new(l, c, title)
    }
    pub(super) fn unexpected(&self, expected: &str) -> Diagnostic {
        let t = self.peek();
        let off = if t.tok == Tok::Eof {
            self.last_end()
        } else {
            t.start
        };
        let mut d = self.err_at(
            off,
            format!("Expected {expected}, but found {}", describe(t)),
        );
        if t.tok == Tok::Eof {
            d = d.explain(format!(
                "The `{}` block ended before {expected}.",
                self.block_kw
            ));
        }
        if t.text == ":=" {
            d = d
                .explain("`:=` is not used in specifications.")
                .hint("Use `=` instead of `:=`.");
        }
        d
    }
    pub(super) fn expect(&mut self, s: &str, what: &str) -> PResult<Token> {
        if self.is(s) {
            Ok(self.next())
        } else {
            Err(self.unexpected(what))
        }
    }
    pub(super) fn ident(&mut self, what: &str) -> PResult<String> {
        let t = self.peek().clone();
        match &t.tok {
            Tok::Ident(n) if bool_word(n).is_none() => {
                self.next();
                Ok(n.clone())
            }
            _ => {
                if let Some(m) = reserved_meaning(&t.text) {
                    return Err(self
                        .err_at(t.start, format!("`{}` is a reserved word and can't be used as a name", t.text))
                        .explain(format!("`{}` is {m}, so it is read as an operator, not as a {what}.", t.text))
                        .hint(match t.text.as_str() {
                            "exists" => "Rename it everywhere (declaration, postcondition and test data), e.g. to `van` or `talalt`.",
                            _ => "Choose another name and rename it everywhere, including the test data.",
                        }));
                }
                Err(self.unexpected(&format!("a {what}")))
            }
        }
    }
    pub(super) fn mk(&self, start: usize, kind: ExprKind) -> Expr {
        let mut e = Expr {
            kind: Box::new(kind),
            start,
            end: self.last_end(),
            depth: 1,
        };
        let inside = e.children().iter().map(|c| c.depth).max().unwrap_or(0);
        if inside >= MAX_DEPTH {
            if self.too_deep.get().is_none() {
                self.too_deep.set(Some(start));
            }
            *e.kind = ExprKind::Bool(false);
        } else {
            e.depth = inside + 1;
        }
        e
    }

    // ---- declarations

    pub(super) fn decl_block(
        &mut self,
        role: Role,
        decls: &mut Vec<Decl>,
        aliases: &mut HashMap<String, Type>,
    ) -> PResult<()> {
        if self.peek().tok == Tok::Eof {
            return Err(self.unexpected("at least one declaration such as `n∈N`"));
        }
        loop {
            let t = self.peek().clone();
            let name = self.ident("variable name")?;
            let (line, col) = line_col(self.src, t.start);
            if self.eat("∈") {
                let ty = self.ty()?;
                let init = if self.eat("=") {
                    Some(self.expr()?)
                } else {
                    None
                };
                if decls.iter().any(|d| d.name == name) {
                    return Err(self
                        .err_at(t.start, format!("`{name}` is declared twice"))
                        .hint("Remove or rename one of the declarations."));
                }
                decls.push(Decl {
                    name,
                    role,
                    ty,
                    init,
                    line,
                    col,
                });
            } else if self.eat("=") {
                if !name.starts_with(|c: char| c.is_uppercase()) {
                    return Err(self
                        .err_at(t.start, format!("`{name}` looks like a variable but is defined with `=`"))
                        .explain("`Name=Type` defines a new type (type names start with an uppercase letter). Variables are declared with `∈`.")
                        .hint(format!("Write `{name}∈…` to declare a variable, or capitalise the name to define a type.")));
                }
                let ty = self.ty()?;
                aliases.insert(name, ty);
            } else if self.is(":") {
                return Err(self
                    .err_at(self.peek().start, "Use `∈` to declare a variable")
                    .explain("`:` is only used for record fields like `(name:S x age:N)` and function signatures.")
                    .hint(format!("Write `{name}∈…` (or `{name} eleme …`).")));
            } else {
                return Err(self.unexpected(&format!(
                    "`∈` after `{name}` (a declaration looks like `{name}∈N`)"
                )));
            }
            if self.eat(",") {
                if self.peek().tok == Tok::Eof {
                    return Err(self
                        .err_at(self.toks[self.pos - 1].start, "Trailing comma")
                        .explain("The last declaration in a block must not be followed by a comma.")
                        .hint("Delete this comma."));
                }
                continue;
            }
            if self.peek().tok == Tok::Eof {
                return Ok(());
            }
            let mut d = self.unexpected("`,` between declarations");
            if matches!(self.peek().tok, Tok::Ident(_)) {
                d = d.hint("Put a comma at the end of the previous declaration.");
            }
            return Err(d);
        }
    }

    fn ty(&mut self) -> PResult<Type> {
        let t = self.peek().clone();
        let base = match &t.tok {
            Tok::Ident(n) => {
                self.next();
                match n.as_str() {
                    "N" => Type::Nat,
                    "Z" => Type::Int,
                    "R" => Type::Real,
                    "L" => Type::Bool,
                    "S" => Type::Str,
                    "C" | "K" => Type::Char,
                    _ if n.starts_with(|c: char| c.is_uppercase()) => Type::Named(n.clone()),
                    _ => {
                        return Err(self
                            .err_at(t.start, format!("`{n}` is not a type"))
                            .explain("Sets/types start with an uppercase letter: N, Z, R, L, S, C (K) or a name you defined like `Student=(name:S x grade:N)`.")
                            .hint(did_you_mean(&n.to_uppercase(), ["N", "Z", "R", "L", "S", "C", "K"]).trim().to_string()));
                    }
                }
            }
            Tok::Sym("[") => {
                self.next();
                let lo = self.add()?;
                self.expect("..", "`..` in the interval `[from..to]`")?;
                let hi = self.add()?;
                self.expect("]", "`]` to close the interval")?;
                Type::Interval(lo, hi)
            }
            Tok::Sym("(") => {
                self.next();
                let mut fields = Vec::new();
                loop {
                    let f = self.ident("record field name")?;
                    self.expect(
                        ":",
                        &format!("`:` after the field name `{f}` (fields look like `{f}:N`)"),
                    )?;
                    let ft = self.nested(Self::ty)?;
                    fields.push((f, ft));
                    if self.eat_product() {
                        continue;
                    }
                    if self.is(",") {
                        return Err(self
                            .err_at(
                                self.peek().start,
                                "Record fields are separated by ` x `, not commas",
                            )
                            .explain("A record is a direct product of its fields.")
                            .hint("Write it like `(name:S x grade:N)`."));
                    }
                    self.expect(
                        ")",
                        "` x ` before the next field or `)` to close the record",
                    )?;
                    break;
                }
                Type::Record(fields)
            }
            _ => {
                return Err(
                    self.unexpected("a type such as N, Z, R, L, S, C, [1..5] or (field:T x …)")
                );
            }
        };
        // array/matrix suffixes: collect all dimensions outer→inner, then wrap
        let mut dims: Vec<(Expr, Option<Expr>)> = Vec::new();
        while self.is("[") {
            let open = self.next();
            let one = |p: &Self| Expr {
                kind: Box::new(ExprKind::Int(1)),
                start: p.last_end(),
                end: p.last_end(),
                depth: 1,
            };
            if self.eat("]") {
                dims.push((one(self), None));
                continue;
            }
            loop {
                if self.is(",") || self.is("]") {
                    dims.push((one(self), None));
                } else {
                    let lo = self.add()?;
                    self.expect("..", "`..` in the index range (like `[1..n]`)")?;
                    let hi = if self.is(",") || self.is("]") {
                        None
                    } else {
                        Some(self.add()?)
                    };
                    dims.push((lo, hi));
                }
                if self.eat(",") {
                    continue;
                }
                if !self.is("]") {
                    let (l, c) = line_col(self.src, open.start);
                    return Err(
                        self.unexpected(&format!("`]` to close the `[` at line {l}, column {c}"))
                    );
                }
                self.next();
                break;
            }
        }
        if dims.len() > MAX_NESTING {
            return Err(self.nesting_error(t.start));
        }
        Ok(dims
            .into_iter()
            .rev()
            .fold(base, |elem, (lo, hi)| Type::Array {
                lo,
                hi,
                elem: Box::new(elem),
            }))
    }

    fn eat_product(&mut self) -> bool {
        let found = matches!(&self.peek().tok, Tok::Ident(x) if x == "x") || self.is("*");
        if found {
            self.next();
        }
        found
    }

    fn type_product(&mut self) -> PResult<()> {
        loop {
            self.ty()?;
            if !self.eat_product() {
                return Ok(());
            }
        }
    }

    // ---- functions

    /// One function per header, as in the online editor: `name:Dom->Cod, name(params)=body`.
    pub(super) fn fn_block(&mut self, funcs: &mut HashMap<String, Func>) -> PResult<()> {
        let t = self.peek().clone();
        let name = self.ident("function name")?;
        if self.is("(") {
            return Err(self
                .err_at(self.peek().start, format!("`{name}` has no signature"))
                .explain(
                    "Each function starts with its signature, then a comma and the definition.",
                )
                .hint(format!("Write `{name}:N->N, {name}(…)=…`.")));
        }
        self.expect(":", &format!("`:` and the signature (like `{name}:N->N`)"))?;
        // signature: types joined by x, ->, types joined by x
        self.type_product()?;
        self.expect(
            "->",
            &format!("`->` between the domain and the codomain (like `{name}:N->L`)"),
        )?;
        self.type_product()?;
        if !self.eat(",") {
            return Err(self.unexpected(&format!("`,` and the definition (like `, {name}(i)=…`)")));
        }
        let dt = self.peek().clone();
        let def = self.ident("function name")?;
        if def != name {
            return Err(self
                .err_at(
                    dt.start,
                    format!("The signature is for `{name}`, but this defines `{def}`"),
                )
                .explain(
                    "Each `Fn:` header holds one function: its signature, then its definition.",
                )
                .hint(format!(
                    "Write `{name}(…)=…` here, and give `{def}` its own `Fn:` header."
                )));
        }
        self.expect("(", &format!("`(` and the parameters of `{name}`"))?;
        let mut params = Vec::new();
        if !self.is(")") {
            loop {
                params.push(self.ident("parameter name")?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.expect(")", "`)` after the parameters")?;
        if self.is(":=") {
            return Err(self.unexpected("`=`"));
        }
        self.expect("=", &format!("`=` and the definition of `{name}`"))?;
        let body = self.expr()?;
        if self.is(",") {
            return Err(self
                .err_at(self.peek().start, "Only one function per `Fn:` header")
                .explain("The online editor reads a comma-separated list of functions as a syntax error.")
                .hint("Start the next function on a new line with its own `Fn:` header."));
        }
        if self.peek().tok != Tok::Eof {
            return Err(self.unexpected("the end of the function definition"));
        }
        if funcs.contains_key(&name) {
            return Err(self
                .err_at(t.start, format!("`{name}` is defined twice"))
                .hint("Keep only one definition."));
        }
        funcs.insert(name, Func { params, body });
        Ok(())
    }
}
