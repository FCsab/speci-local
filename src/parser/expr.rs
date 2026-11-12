//! Conditions and expressions: the precedence levels from `or` down to the atoms.
//! `->` binds tighter than `and`/`or`, so `p -> a and not p -> b` is two implications (ELTE usage).

use super::ast::{BinOp, Expr, ExprKind, MACROS, line_col, macro_alias};
use super::grammar::{PResult, Parser, bool_word};
use super::lexer::{Tok, Token};

impl Parser<'_> {
    pub(super) fn condition_block(&mut self, pre: bool) -> PResult<Expr> {
        if self.toks.len() == 2 && self.is("-") {
            let t = self.next();
            return Ok(Expr {
                kind: Box::new(ExprKind::Bool(true)),
                start: t.start,
                end: t.end,
                depth: 1,
            });
        }
        if self.peek().tok == Tok::Eof {
            return Err(self.unexpected(if pre {
                "a condition (write `-` if there is no precondition)"
            } else {
                "the postcondition"
            }));
        }
        self.condition()
    }

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

    pub(super) fn expr(&mut self) -> PResult<Expr> {
        self.nested(Self::or)
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
        let mut l = self.implication()?;
        while self.eat("and") {
            let r = self.implication()?;
            l = self.mk(start, ExprKind::Bin(BinOp::And, l, r));
        }
        Ok(l)
    }
    fn implication(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        let l = self.not()?;
        if self.eat("->") {
            let r = self.nested(Self::implication)?;
            return Ok(self.mk(start, ExprKind::Bin(BinOp::Imp, l, r)));
        }
        Ok(l)
    }
    fn not(&mut self) -> PResult<Expr> {
        let start = self.peek().start;
        if self.eat("not") {
            let e = self.nested(Self::not)?;
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
        while let Tok::Sym(s @ ("=" | "!=" | "<" | ">" | "<=" | ">=" | "|" | "∤")) = self.peek().tok
        {
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
    pub(super) fn add(&mut self) -> PResult<Expr> {
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
            } else if self.eat("div") {
                BinOp::IDiv
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
            let e = self.nested(Self::unary)?;
            return Ok(self.mk(start, ExprKind::Neg(e)));
        }
        if self.eat("not") {
            let e = self.nested(Self::unary)?;
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
                    other => {
                        if other != Tok::Eof {
                            self.pos -= 1;
                        }
                        return Err(self.unexpected(
                            "a field name or a tuple position (like `.name` or `.1`) after `.`",
                        ));
                    }
                };
                e = self.mk(start, ExprKind::Field(e, f));
            } else if self.eat("!") {
                e = self.mk(start, ExprKind::Call("!".into(), vec![e]));
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
                if let Some(b) = bool_word(&name) {
                    return Ok(self.mk(start, ExprKind::Bool(b)));
                }
                let canon = macro_alias(&name).unwrap_or(&name);
                if let Some(&(m, min, max, usage)) = MACROS.iter().find(|m| m.0 == canon) {
                    return self.macro_call(start, m.to_string(), min, max, usage);
                }
                let upper = name.to_uppercase();
                if self.is("(") && upper != name && MACROS.iter().any(|m| m.0 == upper) {
                    return Err(self
                        .err_at(start, format!("Unknown function `{name}`"))
                        .doc("ch06-00-theorems.html")
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
            Tok::Sym("(") => self.parenthesized(start),
            Tok::Sym("[") => self.array_literal(start),
            Tok::Sym("{") => self.cases(start),
            Tok::Sym(q @ ("∀" | "∃")) => self.quantifier(&t, q),
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
    /// `{a, ha c; b, ha d; e egyébként}` (`if` / `otherwise` in English); the commas before
    /// `ha` and `egyébként` may be left out.
    fn cases(&mut self, start: usize) -> PResult<Expr> {
        let word = |p: &mut Self, words: [&str; 2]| {
            let hit = matches!(&p.peek().tok, Tok::Ident(w) if words.contains(&w.as_str()));
            if hit {
                p.next();
            }
            hit
        };
        self.next();
        let mut arms = vec![];
        loop {
            let value = self.expr()?;
            self.eat(",");
            if word(self, ["egyébként", "otherwise"]) {
                if !self.eat("}") {
                    return Err(self.unexpected("`}` after `egyébként`"));
                }
                return Ok(self.mk(start, ExprKind::Cases(arms, Some(value))));
            }
            if !word(self, ["ha", "if"]) {
                let (l, c) = line_col(self.src, start);
                return Err(self
                    .unexpected("`ha` and a condition, or `egyébként`")
                    .explain(format!(
                        "The `{{` at line {l}, column {c} starts cases: `{{a, ha c; b egyébként}}`."
                    )));
            }
            let cond = self.expr()?;
            arms.push((value, cond));
            if self.eat("}") {
                return Ok(self.mk(start, ExprKind::Cases(arms, None)));
            }
            if !self.eat(";") {
                return Err(self.unexpected("`;` before the next case, or `}`"));
            }
        }
    }

    fn parenthesized(&mut self, start: usize) -> PResult<Expr> {
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

    fn array_literal(&mut self, start: usize) -> PResult<Expr> {
        self.next();
        let mut items = Vec::new();
        let mut from = Vec::new();
        if !self.eat("]") {
            loop {
                let x = self.expr()?;
                // `[0..:1, 2]`: start indexes, one per dimension, before a `:`
                if items.is_empty() && self.eat("..") {
                    from.push(x);
                    if self.eat(",") {
                        continue;
                    }
                    self.expect(":", "`:` after the index ranges (like `[0..:1, 2, 3]`)")?;
                    if self.eat("]") {
                        break;
                    }
                    continue;
                }
                items.push(x);
                if self.eat(",") {
                    continue;
                }
                if self.eat("]") {
                    break;
                }
                return Err(self.unexpected("`,` or `]` in the array literal"));
            }
        }
        Ok(self.mk(start, ExprKind::Array(items, from)))
    }

    fn quantifier(&mut self, t: &Token, q: &str) -> PResult<Expr> {
        let start = t.start;
        let word = t.text.chars().all(char::is_alphabetic);
        self.next();
        if word && !matches!(self.peek().tok, Tok::Ident(_)) {
            // e.g. `(exists,ind)=SEARCH(…)`: the keyword was meant as a variable
            return Err(self
                .err_at(start, format!("`{}` is a reserved word and can't be used as a name", t.text))
                .doc("ch02-02-lexical.html#reserved-words").explain(format!("`{}` is the ASCII spelling of {q}, so it is read as a quantifier here.", t.text))
                .hint("Rename the variable everywhere (declaration, postcondition and test data), e.g. to `van` or `talalt`."));
        }
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
        let body = self.nested(Self::unary)?;
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
            if self.peek().text == "≥" {
                return Err(self
                    .err_at(self.peek().start, "Expected `>=` but found `≥`")
                    .doc("ch06-05-select.html").explain("Inside SELECT the online editor only accepts `>=`, although `≥` works everywhere else.")
                    .hint("Write `SELECT(i>=1, …)`."));
            }
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
                .doc("ch06-00-theorems.html")
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
