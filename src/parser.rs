//! Lexer, block splitter, parser and static checks for specifications.

use crate::diag::{Diagnostic, did_you_mean, suggest};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------- AST

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: Box<ExprKind>,
    pub start: usize, // byte offsets into Spec::src
    pub end: usize,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Real(f64),
    Str(String),
    Bool(bool),
    Var(String),
    Tuple(Vec<Option<Expr>>),
    Array(Vec<Expr>),
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
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
    ("SELECT", 1, 1, "ind=SELECT(i≥b, condition)"),
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

pub const BUILTINS: &[(&str, usize)] = &[
    ("length", 1),
    ("from", 1),
    ("to", 1),
    ("ABS", 1),
    ("abs", 1),
];

// ---------------------------------------------------------------- lexer

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Int(i64),
    Real(f64),
    Str(String),
    Sym(&'static str),
    Eof,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    start: usize,
    end: usize,
    text: String,
}

const SYMS: &[&str] = &[
    "..", "->", "<=", ">=", "!=", ":=", "=", "<", ">", "+", "-", "*", "/", "%", "|", "(", ")", "[",
    "]", "{", "}", ",", ":", ".",
];

fn word_sym(w: &str) -> Option<&'static str> {
    Some(match w {
        "and" | "és" => "and",
        "or" | "vagy" => "or",
        "not" | "nem" => "not",
        "mod" => "%",
        "element" => "∈",
        "exists" => "∃",
        "forall" => "∀",
        _ => return None,
    })
}

fn glyph_sym(c: char) -> Option<&'static str> {
    Some(match c {
        '∈' => "∈",
        '∀' => "∀",
        '∃' => "∃",
        '≤' => "<=",
        '≥' => ">=",
        '≠' => "!=",
        '→' => "->",
        '∧' => "and",
        '∨' => "or",
        '¬' => "not",
        '–' | '−' => "-", // en dash / minus sign, as in "Pre: –"
        _ => return None,
    })
}

fn lex(src: &str, from: usize, to: usize) -> Result<Vec<Token>, Diagnostic> {
    let s = &src[from..to];
    let mut toks = Vec::new();
    let mut it = s.char_indices().peekable();
    let at = |i: usize| from + i;
    while let Some(&(i, c)) = it.peek() {
        if c.is_whitespace() {
            it.next();
            continue;
        }
        let start = at(i);
        let push = |toks: &mut Vec<Token>, tok, end: usize| {
            toks.push(Token {
                tok,
                start,
                end,
                text: src[start..end].to_string(),
            })
        };
        if c.is_alphabetic() || c == '_' {
            let mut end = i;
            while let Some(&(j, d)) = it.peek() {
                if d.is_alphanumeric() || d == '_' {
                    end = j + d.len_utf8();
                    it.next();
                } else {
                    break;
                }
            }
            let w = &s[i..end];
            let tok = word_sym(w)
                .map(Tok::Sym)
                .unwrap_or_else(|| Tok::Ident(w.to_string()));
            push(&mut toks, tok, at(end));
        } else if c.is_ascii_digit() {
            let mut end = i;
            while let Some(&(j, d)) = it.peek() {
                if d.is_ascii_digit() {
                    end = j + 1;
                    it.next();
                } else {
                    break;
                }
            }
            let rest = &s[end..];
            let mut chars = rest.chars();
            if chars.next() == Some('.') && chars.next().is_some_and(|d| d.is_ascii_digit()) {
                it.next(); // '.'
                end += 1;
                while let Some(&(j, d)) = it.peek() {
                    if d.is_ascii_digit() {
                        end = j + 1;
                        it.next();
                    } else {
                        break;
                    }
                }
                push(&mut toks, Tok::Real(s[i..end].parse().unwrap()), at(end));
            } else {
                let v = s[i..end].parse::<i64>().map_err(|_| {
                    let (l, c) = line_col(src, start);
                    Diagnostic::new(l, c, "Number is too large")
                        .explain("Whole numbers must fit in 64 bits.")
                })?;
                push(&mut toks, Tok::Int(v), at(end));
            }
        } else if c == '"' || c == '\'' || c == '„' || c == '”' {
            it.next();
            let close = if c == '„' { '”' } else { c };
            let mut end = None;
            for (j, d) in it.by_ref() {
                if d == close || (close == '"' && d == '”') {
                    end = Some(j);
                    break;
                }
                if d == '\n' {
                    break;
                }
            }
            let Some(e) = end else {
                let (l, col) = line_col(src, start);
                return Err(Diagnostic::new(l, col, "Unterminated text literal")
                    .explain("A text value starts with a quote but the line ends before the closing quote.")
                    .hint("Add the closing `\"`."));
            };
            let body = s[i + c.len_utf8()..e].to_string();
            let end = e + close.len_utf8();
            push(&mut toks, Tok::Str(body), at(end));
        } else if let Some(sym) = glyph_sym(c) {
            it.next();
            push(&mut toks, Tok::Sym(sym), at(i + c.len_utf8()));
        } else if let Some(sym) = SYMS.iter().find(|p| s[i..].starts_with(**p)) {
            for _ in 0..sym.len() {
                it.next();
            }
            push(&mut toks, Tok::Sym(sym), at(i + sym.len()));
        } else {
            let (l, col) = line_col(src, start);
            return Err(Diagnostic::new(l, col, format!("Unexpected character `{c}`"))
                .explain("This character is not part of the specification language.")
                .hint("Use the symbol buttons (∈ ∀ ∃ ≠ ≤ ≥ →) or their ASCII forms (element, forall, exists, !=, <=, >=, ->)."));
        }
    }
    toks.push(Token {
        tok: Tok::Eof,
        start: to,
        end: to,
        text: String::new(),
    });
    Ok(toks)
}

/// Replace comments with spaces so offsets/lines stay the same.
fn strip_comments(src: &str) -> Result<String, Diagnostic> {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    let mut in_str = false;
    while let Some(c) = rest.chars().next() {
        if c == '\n' {
            in_str = false;
        }
        if c == '"' {
            in_str = !in_str;
        }
        if !in_str && rest.starts_with("//") {
            let n = rest.find('\n').unwrap_or(rest.len());
            out.extend(std::iter::repeat_n(' ', n));
            rest = &rest[n..];
        } else if !in_str && rest.starts_with("/*") {
            let Some(n) = rest[2..].find("*/").map(|n| n + 4) else {
                let (l, col) = line_col(src, src.len() - rest.len());
                return Err(Diagnostic::new(l, col, "Comment is never closed")
                    .explain("A `/*` block comment runs until `*/`, but there is no `*/` after this point.")
                    .hint("Add `*/` where the comment should end."));
            };
            for ch in rest[..n].chars() {
                // keep byte length identical: pad multi-byte chars with spaces
                if ch == '\n' {
                    out.push('\n')
                } else {
                    out.extend(std::iter::repeat_n(' ', ch.len_utf8()))
                }
            }
            rest = &rest[n..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- blocks

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    In,
    Aux,
    Out,
    Fn,
    Pre,
    Post,
}

const HEADERS: &[(&str, Kind)] = &[
    ("In", Kind::In),
    ("Be", Kind::In),
    ("Aux", Kind::Aux),
    ("Sa", Kind::Aux),
    ("Out", Kind::Out),
    ("Ki", Kind::Out),
    ("Fn", Kind::Fn),
    ("Fv", Kind::Fn),
    ("Pre", Kind::Pre),
    ("Ef", Kind::Pre),
    ("Post", Kind::Post),
    ("Uf", Kind::Post),
];

/// Plausible but wrong header spellings → the real one.
const WRONG_HEADERS: &[(&str, &str)] = &[
    ("segéd", "Sa:"),
    ("seged", "Sa:"),
    ("segedvaltozo", "Sa:"),
    ("bemenet", "Be:"),
    ("kimenet", "Ki:"),
    ("függvény", "Fv:"),
    ("fuggveny", "Fv:"),
    ("előfeltétel", "Ef:"),
    ("elofeltetel", "Ef:"),
    ("utófeltétel", "Uf:"),
    ("utofeltetel", "Uf:"),
    ("input", "In:"),
    ("output", "Out:"),
    ("auxiliary", "Aux:"),
    ("function", "Fn:"),
    ("functions", "Fn:"),
    ("precondition", "Pre:"),
    ("postcondition", "Post:"),
];

fn kind_name(k: Kind, hu: bool) -> &'static str {
    let i = HEADERS.iter().position(|(_, kk)| *kk == k).unwrap();
    HEADERS[i + hu as usize].0
}

struct Block {
    kind: Kind,
    kw: String,
    line: usize,
    body: (usize, usize),
}

fn split_blocks(src: &str) -> Result<Vec<Block>, Diagnostic> {
    let all_headers = "In: Be: Aux: Sa: Out: Ki: Fn: Fv: Pre: Ef: Post: Uf:";
    let mut blocks: Vec<Block> = Vec::new();
    let mut offset = 0;
    let mut first_content: Option<usize> = None;
    for (idx, line) in src.split('\n').enumerate() {
        let line_no = idx + 1;
        let lstart = offset;
        offset += line.len() + 1;
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let word: String = trimmed
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let after = &trimmed[word.len()..];
        let col = line[..indent].chars().count() + 1;
        let colon_follows = after.trim_start_matches([' ', '\t']).starts_with(':')
            && !after.trim_start().starts_with(":=");
        let exact = HEADERS.iter().find(|(h, _)| *h == word);
        if let Some((h, kind)) =
            exact.filter(|_| after.starts_with(':') && !after.starts_with(":="))
        {
            let rest = &after[1..];
            if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
                return Err(Diagnostic::new(line_no, col + word.chars().count() + 1, format!("Missing space after `{h}:`"))
                    .explain("A block header is followed either by a space (the content continues on the same line) or by a line break (the content starts on the next line). The ELTE editor rejects anything else.")
                    .hint(format!("Write `{h}: {}`", rest.trim())));
            }
            let body_start = lstart + indent + word.len() + 1;
            if let Some(last) = blocks.last_mut() {
                last.body.1 = lstart;
            } else if let Some(fc) = first_content {
                let (l, c) = line_col(src, fc);
                return Err(Diagnostic::new(l, c, "Text before the first block")
                    .explain("A specification must start with the input block. Only comments may come before it.")
                    .hint("Start with `In:` (or `Be:`), or turn this text into a comment with `//`."));
            }
            blocks.push(Block {
                kind: *kind,
                kw: format!("{h}:"),
                line: line_no,
                body: (body_start, src.len()),
            });
            continue;
        }
        if !word.is_empty() && colon_follows {
            let lower = word.to_lowercase();
            let wrong = WRONG_HEADERS
                .iter()
                .find(|(w, _)| *w == lower)
                .map(|(_, r)| *r);
            let case_twin = HEADERS
                .iter()
                .find(|(h, _)| h.to_lowercase() == lower)
                .map(|(h, _)| *h);
            let looks_like_header = word.chars().next().unwrap().is_uppercase()
                || wrong.is_some()
                || case_twin.is_some();
            if looks_like_header {
                let mut d =
                    Diagnostic::new(line_no, col, format!("Unknown block header `{word}:`"));
                if let Some(h) = case_twin.filter(|h| *h == word) {
                    d = d
                        .explain(format!(
                            "There must be no space between `{h}` and the colon."
                        ))
                        .hint(format!("Write `{h}:`"));
                } else if let Some(h) = case_twin {
                    d = d
                        .explain(format!(
                            "Block headers are case-sensitive. The valid headers are: {all_headers}"
                        ))
                        .hint(format!("Write `{h}:`"));
                } else if let Some(r) = wrong {
                    d = d
                        .explain(format!("`{word}:` looks like a translation of a header, but the editor only knows these: {all_headers}"))
                        .hint(format!("Write `{r}`"));
                } else {
                    let s = suggest(&word, HEADERS.iter().map(|(h, _)| *h));
                    d = d.explain(format!("The valid block headers are: {all_headers}"));
                    if let Some(s) = s {
                        d = d.hint(format!("Did you mean `{s}:`?"));
                    }
                }
                return Err(d);
            }
        }
        if blocks.is_empty() && first_content.is_none() && !trimmed.trim().is_empty() {
            first_content = Some(lstart + indent);
        }
    }
    if blocks.is_empty() {
        if let Some(fc) = first_content {
            let (l, c) = line_col(src, fc);
            return Err(Diagnostic::new(l, c, "No block headers found")
                .explain(format!(
                    "A specification consists of blocks that start with a header: {all_headers}"
                ))
                .hint("Start with `In: n∈N` (or `Be:` on its own line)."));
        }
        return Err(Diagnostic::plain("The specification is empty")
            .hint("Start with an `In:` / `Be:` block."));
    }

    // order and presence
    let hu = blocks[0].kw == "Be:"
        || blocks
            .iter()
            .any(|b| matches!(b.kw.as_str(), "Be:" | "Ki:" | "Ef:" | "Uf:"));
    let rank = |k: Kind| match k {
        Kind::In | Kind::Aux | Kind::Out => 0,
        Kind::Fn => 1,
        Kind::Pre => 2,
        Kind::Post => 3,
    };
    let mut seen: Vec<&Block> = Vec::new();
    for b in &blocks {
        if let Some(prev) = seen.iter().find(|p| p.kind == b.kind) {
            return Err(Diagnostic::new(b.line, 1, format!("Second `{}` block", b.kw))
                .explain(format!("This kind of block was already started at line {} (`{}`). Each block may appear only once.", prev.line, prev.kw))
                .hint("Merge the two blocks: separate the items with commas."));
        }
        if let Some(later) = seen.iter().find(|p| rank(p.kind) > rank(b.kind)) {
            let what = if rank(b.kind) == 0 {
                "Declarations (In/Aux/Out) come first, then Fn, then Pre, then Post."
            } else {
                "The order is: declarations (In/Aux/Out), then Fn, then Pre, then Post."
            };
            return Err(
                Diagnostic::new(b.line, 1, format!("`{}` is in the wrong place", b.kw))
                    .explain(format!(
                        "It comes after `{}` (line {}). {what}",
                        later.kw, later.line
                    ))
                    .hint(format!("Move the `{}` block before `{}`.", b.kw, later.kw)),
            );
        }
        seen.push(b);
    }
    for (k, why) in [
        (Kind::In, "it declares the input variables"),
        (Kind::Out, "it declares the output variables"),
        (
            Kind::Pre,
            "write `- ` as its content if there is no precondition",
        ),
        (Kind::Post, "it states what the program must achieve"),
    ] {
        if !blocks.iter().any(|b| b.kind == k) {
            let name = kind_name(k, hu);
            let mut d = Diagnostic::plain(format!("Missing `{name}:` block"))
                .explain(format!("This block is compulsory: {why}."));
            if k == Kind::Pre {
                d = d.hint(format!("Add `{name}: -`"));
            }
            return Err(d);
        }
    }
    Ok(blocks)
}

// ---------------------------------------------------------------- parser

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
    block_kw: String,
}

type PResult<T> = Result<T, Diagnostic>;

fn describe(t: &Token) -> String {
    match &t.tok {
        Tok::Eof => "the end of the block".into(),
        Tok::Str(_) => format!("the text {}", t.text),
        _ => format!("`{}`", t.text),
    }
}

fn reserved_meaning(word: &str) -> Option<&'static str> {
    Some(match word {
        "exists" => "the ASCII spelling of ∃",
        "forall" => "the ASCII spelling of ∀",
        "element" => "the ASCII spelling of ∈",
        "and" | "és" => "logical AND",
        "or" | "vagy" => "logical OR",
        "not" | "nem" => "logical NOT",
        "mod" => "the remainder operator",
        "true" | "false" => "a logical constant",
        _ => return None,
    })
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }
    fn next(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn is(&self, s: &str) -> bool {
        matches!(&self.peek().tok, Tok::Sym(x) if *x == s)
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.is(s) {
            self.next();
            true
        } else {
            false
        }
    }
    fn last_end(&self) -> usize {
        if self.pos == 0 {
            self.toks[0].start
        } else {
            self.toks[self.pos - 1].end
        }
    }
    fn err_at(&self, offset: usize, title: impl Into<String>) -> Diagnostic {
        let (l, c) = line_col(self.src, offset);
        Diagnostic::new(l, c, title)
    }
    fn unexpected(&self, expected: &str) -> Diagnostic {
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
    fn expect(&mut self, s: &str, what: &str) -> PResult<Token> {
        if self.is(s) {
            Ok(self.next())
        } else {
            Err(self.unexpected(what))
        }
    }
    fn ident(&mut self, what: &str) -> PResult<String> {
        let t = self.peek().clone();
        match &t.tok {
            Tok::Ident(n) if !matches!(n.as_str(), "true" | "false") => {
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
    fn mk(&self, start: usize, kind: ExprKind) -> Expr {
        Expr {
            kind: Box::new(kind),
            start,
            end: self.last_end(),
        }
    }

    // ---- declarations

    fn decl_block(
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
                    .hint(format!("Write `{name}∈…` (or `{name} element …`).")));
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
                    let ft = self.ty()?;
                    fields.push((f, ft));
                    if matches!(&self.peek().tok, Tok::Ident(x) if x == "x") || self.is("*") {
                        self.next();
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
        Ok(dims
            .into_iter()
            .rev()
            .fold(base, |elem, (lo, hi)| Type::Array {
                lo,
                hi,
                elem: Box::new(elem),
            }))
    }

    // ---- functions

    fn fn_block(&mut self, funcs: &mut HashMap<String, Func>) -> PResult<()> {
        let mut sigs: Vec<(String, usize)> = Vec::new();
        loop {
            let t = self.peek().clone();
            let name = self.ident("function name")?;
            if self.eat(":") {
                // signature: types joined by x, ->, types joined by x
                loop {
                    self.ty()?;
                    if matches!(&self.peek().tok, Tok::Ident(x) if x == "x") || self.is("*") {
                        self.next();
                        continue;
                    }
                    break;
                }
                self.expect(
                    "->",
                    &format!("`->` between the domain and the codomain (like `{name}:N->L`)"),
                )?;
                loop {
                    self.ty()?;
                    if matches!(&self.peek().tok, Tok::Ident(x) if x == "x") || self.is("*") {
                        self.next();
                        continue;
                    }
                    break;
                }
                sigs.push((name, t.start));
            } else if self.eat("(") {
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
                if funcs.contains_key(&name) {
                    return Err(self
                        .err_at(t.start, format!("`{name}` is defined twice"))
                        .hint("Keep only one definition."));
                }
                funcs.insert(name, Func { params, body });
            } else {
                return Err(self.unexpected(&format!(
                    "`:` (signature, like `{name}:N->N`) or `(` (definition, like `{name}(i)=…`)"
                )));
            }
            if self.eat(",") {
                continue;
            }
            if self.peek().tok == Tok::Eof {
                break;
            }
            return Err(self.unexpected("`,` between the signature and the definition"));
        }
        for (name, off) in sigs {
            if !funcs.contains_key(&name) {
                return Err(self
                    .err_at(
                        off,
                        format!("Function `{name}` has a signature but no definition"),
                    )
                    .hint(format!(
                        "Add the definition after a comma, e.g. `, {name}(i)=…`"
                    )));
            }
        }
        Ok(())
    }

    // ---- expressions

    fn condition(&mut self) -> PResult<Expr> {
        let e = self.expr()?;
        if self.peek().tok != Tok::Eof {
            let mut d = self.unexpected("`and`, `or`, `->` or the end of the condition");
            if !matches!(self.peek().tok, Tok::Sym(")")) {
                d = d
                    .explain("The condition before this point is already complete.")
                    .hint("If this starts another condition, join them with `and` (or `és`).");
            } else {
                d = d.explain("This `)` has no matching `(`.");
            }
            return Err(d);
        }
        Ok(e)
    }

    fn expr(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let l = self.or()?;
        if self.eat("->") {
            let r = self.expr()?;
            return Ok(self.mk(start, ExprKind::Bin(BinOp::Imp, l, r)));
        }
        Ok(l)
    }
    fn or(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let mut l = self.and()?;
        while self.eat("or") {
            let r = self.and()?;
            l = self.mk(start, ExprKind::Bin(BinOp::Or, l, r));
        }
        Ok(l)
    }
    fn and(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let mut l = self.not()?;
        while self.eat("and") {
            let r = self.not()?;
            l = self.mk(start, ExprKind::Bin(BinOp::And, l, r));
        }
        Ok(l)
    }
    fn not(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        if self.eat("not") {
            let e = self.not()?;
            return Ok(self.mk(start, ExprKind::Not(e)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let first = self.add()?;
        if self.eat("∈") {
            self.expect("[", "`[` — membership is written `x∈[from..to]`")?;
            let lo = self.add()?;
            self.expect("..", "`..` in `[from..to]`")?;
            let hi = self.add()?;
            self.expect("]", "`]` to close the interval")?;
            return Ok(self.mk(start, ExprKind::In(first, lo, hi)));
        }
        let mut operands = vec![first];
        let mut ops = Vec::new();
        while let Tok::Sym(s @ ("=" | "!=" | "<" | ">" | "<=" | ">=" | "|")) = self.peek().tok {
            self.next();
            ops.push(s);
            operands.push(self.add()?);
        }
        if self.is(":=") {
            return Err(self.unexpected("an operator"));
        }
        if ops.is_empty() {
            return Ok(operands.pop().unwrap());
        }
        Ok(self.mk(start, ExprKind::Cmp(operands, ops)))
    }
    fn add(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let mut l = self.mul()?;
        loop {
            let op = if self.eat("+") {
                BinOp::Add
            } else if self.eat("-") {
                BinOp::Sub
            } else {
                return Ok(l);
            };
            let r = self.mul()?;
            l = self.mk(start, ExprKind::Bin(op, l, r));
        }
    }
    fn mul(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let mut l = self.unary()?;
        loop {
            let op = if self.eat("*") {
                BinOp::Mul
            } else if self.eat("/") {
                BinOp::Div
            } else if self.eat("%") {
                BinOp::Mod
            } else {
                return Ok(l);
            };
            let r = self.unary()?;
            l = self.mk(start, ExprKind::Bin(op, l, r));
        }
    }
    fn unary(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        if self.eat("-") {
            let e = self.unary()?;
            return Ok(self.mk(start, ExprKind::Neg(e)));
        }
        if self.eat("not") {
            let e = self.unary()?;
            return Ok(self.mk(start, ExprKind::Not(e)));
        }
        let mut e = self.atom()?;
        loop {
            if self.is("[") {
                let open = self.next();
                let mut idx = vec![self.expr()?];
                while self.eat(",") {
                    idx.push(self.expr()?);
                }
                if !self.eat("]") {
                    let (l, c) = line_col(self.src, open.start);
                    return Err(
                        self.unexpected(&format!("`]` to close the `[` at line {l}, column {c}"))
                    );
                }
                e = self.mk(start, ExprKind::Index(e, idx));
            } else if self.eat(".") {
                let t = self.next();
                let f = match t.tok {
                    Tok::Ident(n) => n,
                    Tok::Int(i) => i.to_string(),
                    _ => {
                        self.pos -= 1;
                        return Err(self.unexpected(
                            "a field name or a tuple position (like `.name` or `.1`) after `.`",
                        ));
                    }
                };
                e = self.mk(start, ExprKind::Field(e, f));
            } else {
                return Ok(e);
            }
        }
    }
    fn args(&mut self, open: &Token) -> PResult<Vec<Expr>> {
        let mut v = Vec::new();
        if self.eat(")") {
            return Ok(v);
        }
        loop {
            v.push(self.expr()?);
            if self.eat(",") {
                continue;
            }
            if self.eat(")") {
                return Ok(v);
            }
            let (l, c) = line_col(self.src, open.start);
            return Err(self.unexpected(&format!(
                "`,` or `)` to close the `(` at line {l}, column {c}"
            )));
        }
    }
    fn atom(&mut self) -> PResult<Expr> {
        let t = self.peek().clone();
        let start = t.start;
        match t.tok {
            Tok::Int(i) => {
                self.next();
                Ok(self.mk(start, ExprKind::Int(i)))
            }
            Tok::Real(r) => {
                self.next();
                Ok(self.mk(start, ExprKind::Real(r)))
            }
            Tok::Str(s) => {
                self.next();
                Ok(self.mk(start, ExprKind::Str(s)))
            }
            Tok::Ident(name) => {
                self.next();
                if name == "true" || name == "false" {
                    return Ok(self.mk(start, ExprKind::Bool(name == "true")));
                }
                if let Some(&(_, min, max, usage)) = MACROS.iter().find(|m| m.0 == name) {
                    return self.macro_call(start, name, min, max, usage);
                }
                let upper = name.to_uppercase();
                if self.is("(") && upper != name && MACROS.iter().any(|m| m.0 == upper) {
                    return Err(self
                        .err_at(start, format!("Unknown function `{name}`"))
                        .explain("Theorem macros are written in uppercase.")
                        .hint(format!("Write `{upper}(…)`.")));
                }
                if self.is("(") {
                    let open = self.next();
                    let args = self.args(&open)?;
                    return Ok(self.mk(start, ExprKind::Call(name, args)));
                }
                Ok(self.mk(start, ExprKind::Var(name)))
            }
            Tok::Sym("(") => {
                self.next();
                let mut slots: Vec<Option<Expr>> = Vec::new();
                let mut commas = 0;
                loop {
                    if self.is(",") || self.is(")") {
                        slots.push(None);
                    } else {
                        slots.push(Some(self.expr()?));
                    }
                    if self.eat(",") {
                        commas += 1;
                        continue;
                    }
                    if self.eat(")") {
                        break;
                    }
                    let (l, c) = line_col(self.src, start);
                    return Err(self.unexpected(&format!(
                        "`)` to close the `(` opened at line {l}, column {c}"
                    )));
                }
                if commas == 0 {
                    return match slots.pop().unwrap() {
                        Some(mut e) => {
                            e.start = start;
                            e.end = self.last_end();
                            Ok(e)
                        }
                        None => Err(self
                            .err_at(start, "Empty parentheses `()`")
                            .hint("Put an expression inside, or remove them.")),
                    };
                }
                Ok(self.mk(start, ExprKind::Tuple(slots)))
            }
            Tok::Sym("[") => {
                self.next();
                let mut items = Vec::new();
                if !self.eat("]") {
                    loop {
                        items.push(self.expr()?);
                        if self.eat(",") {
                            continue;
                        }
                        if self.eat("]") {
                            break;
                        }
                        return Err(self.unexpected("`,` or `]` in the array literal"));
                    }
                }
                Ok(self.mk(start, ExprKind::Array(items)))
            }
            Tok::Sym(q @ ("∀" | "∃")) => {
                self.next();
                let var = self.ident("quantifier variable (like `i` in `∀i∈[1..n]:(…)`)")?;
                self.expect(
                    "∈",
                    &format!("`∈` after `{q}{var}` (like `{q}{var}∈[1..n]:(…)`)"),
                )?;
                self.expect("[", "`[` to start the range (like `[1..n]`)")?;
                let lo = self.add()?;
                self.expect("..", "`..` in the range")?;
                let hi = self.add()?;
                self.expect("]", "`]` to close the range")?;
                self.expect(
                    ":",
                    &format!("`:` after the range (like `{q}{var}∈[1..n]:(…)`)"),
                )?;
                let body = self.unary()?;
                Ok(self.mk(
                    start,
                    ExprKind::Quant {
                        all: q == "∀",
                        var,
                        lo,
                        hi,
                        body,
                    },
                ))
            }
            _ => {
                let mut d = self.unexpected("a value or an expression");
                if t.tok == Tok::Eof && self.pos > 0 {
                    d = d.explain(format!(
                        "The `{}` block ends with an operator that has nothing after it.",
                        self.block_kw
                    ));
                }
                Err(d)
            }
        }
    }
    fn macro_call(
        &mut self,
        start: usize,
        name: String,
        min: usize,
        max: usize,
        usage: &str,
    ) -> PResult<Expr> {
        if !self.is("(") {
            return Err(self.unexpected(&format!("`(` after {name} — usage: {usage}")));
        }
        let open = self.next();
        let var = self.ident(&format!("loop variable (usage: {usage})"))?;
        let (lo, hi) = if name == "SELECT" {
            if !self.eat(">=") {
                return Err(self.unexpected(&format!("`≥` (or `>=`) — usage: {usage}")));
            }
            (self.add()?, None)
        } else {
            if !self.eat("=") {
                return Err(
                    self.unexpected(&format!("`=` after the loop variable — usage: {usage}"))
                );
            }
            let lo = self.add()?;
            self.expect("..", &format!("`..` in the range — usage: {usage}"))?;
            (lo, Some(self.add()?))
        };
        let mut args = Vec::new();
        while self.eat(",") {
            args.push(self.expr()?);
        }
        if !self.eat(")") {
            let (l, c) = line_col(self.src, open.start);
            return Err(self.unexpected(&format!(
                "`,` or `)` to close the `{name}(` at line {l}, column {c}"
            )));
        }
        if args.len() < min || args.len() > max {
            let want = if min == max {
                format!("{min}")
            } else {
                format!("{min} or {max}")
            };
            return Err(self
                .err_at(
                    start,
                    format!(
                        "{name} got {} argument(s) after the range, but needs {want}",
                        args.len()
                    ),
                )
                .explain(format!("Usage: {usage}")));
        }
        Ok(self.mk(
            start,
            ExprKind::Macro {
                name,
                var,
                lo,
                hi,
                args,
            },
        ))
    }
}

// ---------------------------------------------------------------- entry

pub fn parse(src: &str) -> Result<Spec, Vec<Diagnostic>> {
    let clean = strip_comments(src).map_err(|d| vec![d])?;
    let blocks = split_blocks(&clean).map_err(|d| vec![d])?;
    let mut decls = Vec::new();
    let mut aliases = HashMap::new();
    let mut funcs = HashMap::new();
    let (mut pre, mut post) = (None, None);
    let (mut pre_kw, mut post_kw) = (String::new(), String::new());
    for b in &blocks {
        let toks = lex(&clean, b.body.0, b.body.1).map_err(|d| vec![d])?;
        let mut p = Parser {
            src: &clean,
            toks,
            pos: 0,
            block_kw: b.kw.clone(),
        };
        let r = match b.kind {
            Kind::In => p.decl_block(Role::In, &mut decls, &mut aliases),
            Kind::Aux => p.decl_block(Role::Aux, &mut decls, &mut aliases),
            Kind::Out => p.decl_block(Role::Out, &mut decls, &mut aliases),
            Kind::Fn => p.fn_block(&mut funcs),
            Kind::Pre | Kind::Post => {
                let e = if p.toks.len() == 2 && p.is("-") {
                    let t = p.next();
                    Ok(Expr {
                        kind: Box::new(ExprKind::Bool(true)),
                        start: t.start,
                        end: t.end,
                    })
                } else if p.peek().tok == Tok::Eof {
                    let what = if b.kind == Kind::Pre {
                        "a condition (write `-` if there is no precondition)"
                    } else {
                        "the postcondition"
                    };
                    Err(p.unexpected(what))
                } else {
                    p.condition()
                };
                e.map(|e| {
                    if b.kind == Kind::Pre {
                        pre = Some(e);
                        pre_kw = b.kw.clone();
                    } else {
                        post = Some(e);
                        post_kw = b.kw.clone();
                    }
                })
            }
        };
        r.map_err(|d| vec![d])?;
    }
    let spec = Spec {
        src: src.to_string(),
        decls,
        aliases,
        funcs,
        pre: pre.unwrap(),
        post: post.unwrap(),
        pre_kw,
        post_kw,
    };
    let diags = check(&spec);
    if diags.is_empty() {
        Ok(spec)
    } else {
        Err(diags)
    }
}

// ---------------------------------------------------------------- static checks

fn walk<'e>(e: &'e Expr, f: &mut dyn FnMut(&'e Expr)) {
    f(e);
    let mut go = |x: &'e Expr| walk(x, f);
    match &*e.kind {
        ExprKind::Tuple(s) => s.iter().flatten().for_each(go),
        ExprKind::Array(v) | ExprKind::Call(_, v) => v.iter().for_each(go),
        ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Field(a, _) => go(a),
        ExprKind::Bin(_, a, b) => {
            go(a);
            go(b)
        }
        ExprKind::Cmp(v, _) => v.iter().for_each(go),
        ExprKind::In(a, b, c) => {
            go(a);
            go(b);
            go(c)
        }
        ExprKind::Index(a, v) => {
            go(a);
            v.iter().for_each(go)
        }
        ExprKind::Quant { lo, hi, body, .. } => {
            go(lo);
            go(hi);
            go(body)
        }
        ExprKind::Macro { lo, hi, args, .. } => {
            go(lo);
            if let Some(h) = hi {
                go(h)
            }
            args.iter().for_each(go)
        }
        _ => {}
    }
}

fn check(spec: &Spec) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let at = |off: usize, t: String| {
        let (l, c) = spec.pos(off);
        Diagnostic::new(l, c, t)
    };
    let declared: Vec<&str> = spec.decls.iter().map(|d| d.name.as_str()).collect();

    // scoped identifier check
    fn scoped(
        spec: &Spec,
        e: &Expr,
        scope: &mut Vec<String>,
        out: &mut Vec<(usize, String, bool)>,
    ) {
        match &*e.kind {
            ExprKind::Var(n) => {
                if !scope.contains(n) && spec.decl(n).is_none() {
                    out.push((e.start, n.clone(), false));
                }
            }
            ExprKind::Call(n, args) => {
                let known = BUILTINS
                    .iter()
                    .find(|b| b.0 == n)
                    .map(|b| b.1)
                    .or_else(|| spec.funcs.get(n).map(|f| f.params.len()));
                match known {
                    None => out.push((e.start, n.clone(), true)),
                    Some(k) if k != args.len() => {
                        out.push((e.start, format!("{n}\u{0}{k}\u{0}{}", args.len()), true))
                    }
                    _ => {}
                }
                for a in args {
                    scoped(spec, a, scope, out);
                }
            }
            ExprKind::Quant {
                var, lo, hi, body, ..
            } => {
                scoped(spec, lo, scope, out);
                scoped(spec, hi, scope, out);
                scope.push(var.clone());
                scoped(spec, body, scope, out);
                scope.pop();
            }
            ExprKind::Macro {
                var, lo, hi, args, ..
            } => {
                scoped(spec, lo, scope, out);
                if let Some(h) = hi {
                    scoped(spec, h, scope, out);
                }
                scope.push(var.clone());
                for a in args {
                    scoped(spec, a, scope, out);
                }
                scope.pop();
            }
            _ => {
                for c in direct_children(e) {
                    scoped(spec, c, scope, out);
                }
            }
        }
    }

    let mut bad = Vec::new();
    for (name, f) in &spec.funcs {
        let mut scope = f.params.clone();
        let _ = name;
        scoped(spec, &f.body, &mut scope, &mut bad);
    }
    scoped(spec, &spec.pre, &mut Vec::new(), &mut bad);
    scoped(spec, &spec.post, &mut Vec::new(), &mut bad);
    for d in &spec.decls {
        if let Some(i) = &d.init {
            scoped(spec, i, &mut Vec::new(), &mut bad);
        }
    }
    let mut seen = HashSet::new();
    for (off, name, is_call) in bad {
        if !seen.insert((off, name.clone())) {
            continue;
        }
        if is_call {
            if let [n, want, got] = name.split('\0').collect::<Vec<_>>()[..] {
                diags.push(at(
                    off,
                    format!("`{n}` takes {want} argument(s) but gets {got} here"),
                ));
                continue;
            }
            let mut d = at(off, format!("Unknown function `{name}`"));
            if MACROS.iter().any(|m| m.0 == name.to_uppercase()) {
                d = d
                    .explain("Theorem macros are written in uppercase.")
                    .hint(format!("Write `{}(…)`.", name.to_uppercase()));
            } else {
                let cands: Vec<&str> = spec
                    .funcs
                    .keys()
                    .map(|s| s.as_str())
                    .chain(BUILTINS.iter().map(|b| b.0))
                    .collect();
                d = d.explain(format!(
                    "Functions must be defined in the `Fn:`/`Fv:` block. Built-in ones: length, from, to, ABS.{}",
                    did_you_mean(&name, cands)
                ));
            }
            diags.push(d);
        } else {
            let mut d = at(off, format!("`{name}` is not declared"));
            let s = did_you_mean(&name, declared.iter().copied());
            d = d.explain(format!(
                "Every variable must be declared in an In/Aux/Out (Be/Sa/Ki) block.{s}"
            ));
            if s.is_empty() {
                d = d.hint(format!(
                    "Add `{name}∈…` to the right block, or check the spelling."
                ));
            }
            diags.push(d);
        }
    }

    // unknown named types
    fn types<'t>(t: &'t Type, out: &mut Vec<&'t str>) {
        match t {
            Type::Named(n) => out.push(n),
            Type::Array { elem, .. } => types(elem, out),
            Type::Record(fs) => fs.iter().for_each(|(_, t)| types(t, out)),
            _ => {}
        }
    }
    let mut used = Vec::new();
    for d in &spec.decls {
        types(&d.ty, &mut used);
    }
    for t in spec.aliases.values() {
        types(t, &mut used);
    }
    for n in used {
        if !spec.aliases.contains_key(n) && seen.insert((0, n.to_string())) {
            let cands: Vec<&str> = spec.aliases.keys().map(|s| s.as_str()).collect();
            let off = spec.src.find(n).unwrap_or(0);
            diags.push(at(off, format!("Unknown type `{n}`")).explain(format!(
                "Define it in a declaration block, e.g. `{n}=(name:S x value:N)`. Built-in sets: N, Z, R, L, S, C, K.{}",
                did_you_mean(n, cands)
            )));
        }
    }

    // All Variables Mandate: every input appears in Post (directly or via called functions)
    let mut in_post: HashSet<&str> = HashSet::new();
    let mut fn_todo: Vec<&str> = Vec::new();
    let mut fn_done: HashSet<&str> = HashSet::new();
    fn collect<'s>(e: &'s Expr, in_post: &mut HashSet<&'s str>, todo: &mut Vec<&'s str>) {
        walk(e, &mut |x| match &*x.kind {
            ExprKind::Var(n) => {
                in_post.insert(n.as_str());
            }
            ExprKind::Call(n, _) => todo.push(n.as_str()),
            _ => {}
        })
    }
    collect(&spec.post, &mut in_post, &mut fn_todo);
    while let Some(f) = fn_todo.pop() {
        if fn_done.insert(f)
            && let Some(func) = spec.funcs.get(f)
        {
            collect(&func.body, &mut in_post, &mut fn_todo);
        }
    }
    let hu = spec.post_kw == "Uf:";
    let (inkw, postkw, prekw) = if hu {
        ("Be:", "Uf:", "Ef:")
    } else {
        ("In:", "Post:", "Pre:")
    };
    for d in spec.decls.iter().filter(|d| d.role == Role::In) {
        if !in_post.contains(d.name.as_str()) {
            diags.push(
                Diagnostic::new(d.line, d.col, format!("Input `{}` is never used in the postcondition", d.name))
                    .explain(format!(
                        "Every variable declared in `{inkw}` must appear in `{postkw}` (directly or inside a function it calls). Using it only in `{prekw}` doesn't count. The ELTE editor reports this as \"{} is missing from the post condition!\".",
                        d.name
                    ))
                    .hint(format!("If it really isn't needed, append `and {0}={0}` to the postcondition.", d.name)),
            );
        }
    }
    diags.sort_by_key(|d| (d.line, d.col));
    diags
}

fn direct_children(e: &Expr) -> Vec<&Expr> {
    match &*e.kind {
        ExprKind::Tuple(s) => s.iter().flatten().collect(),
        ExprKind::Array(v) | ExprKind::Call(_, v) | ExprKind::Cmp(v, _) => v.iter().collect(),
        ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::Field(a, _) => vec![a],
        ExprKind::Bin(_, a, b) => vec![a, b],
        ExprKind::In(a, b, c) => vec![a, b, c],
        ExprKind::Index(a, v) => std::iter::once(a).chain(v.iter()).collect(),
        ExprKind::Quant { lo, hi, body, .. } => vec![lo, hi, body],
        ExprKind::Macro { lo, hi, args, .. } => std::iter::once(lo)
            .chain(hi.iter())
            .chain(args.iter())
            .collect(),
        _ => vec![],
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
