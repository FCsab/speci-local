//! Comments and tokens.

use super::ast::line_col;
use crate::diag::Diagnostic;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tok {
    Ident(String),
    Int(i64),
    Real(f64),
    Str(String),
    Sym(&'static str),
    Eof,
}

#[derive(Clone, Debug)]
pub(super) struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

const SYMS: &[&str] = &[
    "..", "->", "<=", ">=", "!=", "<>", "!", ":=", "=", "<", ">", "+", "-", "*", "/", "%", "|",
    "(", ")", "[", "]", "{", "}", ",", ":", ".",
];

fn word_sym(w: &str) -> Option<&'static str> {
    Some(match w {
        "and" | "és" => "and",
        "or" | "vagy" => "or",
        "not" | "nem" => "not",
        "mod" => "%",
        "div" => "div",
        "eleme" | "in" => "∈",
        "exists" | "létezik" => "∃",
        "forall" | "minden" => "∀",
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
        '∤' => "∤",
        '¬' => "not",
        '–' | '−' => "-", // en dash / minus sign, as in "Pre: –"
        _ => return None,
    })
}

pub(super) fn lex(src: &str, orig: &str, from: usize, to: usize) -> Result<Vec<Token>, Diagnostic> {
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
                    let (l, c) = line_col(orig, start);
                    Diagnostic::new(l, c, "Number is too large")
                        .doc("ch02-02-lexical.html")
                        .explain("Whole numbers must fit in 64 bits.")
                })?;
                push(&mut toks, Tok::Int(v), at(end));
            }
        } else if let Some(closing) = closing_quotes(c) {
            it.next();
            let close = it
                .by_ref()
                .take_while(|(_, d)| *d != '\n')
                .find(|(_, d)| closing.contains(d));
            let Some((e, quote)) = close else {
                let (l, col) = line_col(orig, start);
                return Err(Diagnostic::new(l, col, "Unterminated text literal")
                    .doc("ch02-02-lexical.html").explain("A text value starts with a quote but the line ends before the closing quote.")
                    .hint("Add the closing `\"`."));
            };
            let body = s[i + c.len_utf8()..e].to_string();
            push(&mut toks, Tok::Str(body), at(e + quote.len_utf8()));
        } else if let Some(sym) = glyph_sym(c) {
            it.next();
            push(&mut toks, Tok::Sym(sym), at(i + c.len_utf8()));
        } else if let Some(sym) = SYMS.iter().find(|p| s[i..].starts_with(**p)) {
            for _ in 0..sym.len() {
                it.next();
            }
            let sym = if *sym == "<>" { "!=" } else { sym };
            push(&mut toks, Tok::Sym(sym), at(i + sym.len()));
        } else {
            let (l, col) = line_col(orig, start);
            let mut d = Diagnostic::new(l, col, format!("Unexpected character `{c}`"))
                .doc("appendix-02-symbols.html")
                .explain("This character is not part of the specification language.");
            d = match c {
                '→' => d.hint("Write the arrow as `->`. The online editor rejects `→`."),
                '∧' => d.hint("Write `and` (or `és`)."),
                '∨' => d.hint("Write `or` (or `vagy`)."),
                _ => d.hint("Use the symbol buttons (∈ ∀ ∃ ≠ ≤ ≥) or their word forms (in, forall, exists, !=, <=, >=)."),
            };
            return Err(d);
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

pub fn closing_quotes(open: char) -> Option<&'static [char]> {
    match open {
        '"' => Some(&['"', '”']),
        '\'' => Some(&['\'']),
        '„' | '”' => Some(&['”']),
        _ => None,
    }
}

/// Replace comments with spaces so offsets/lines stay the same.
pub(super) fn strip_comments(src: &str) -> Result<String, Diagnostic> {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    let mut closing: Option<&[char]> = None;
    while let Some(c) = rest.chars().next() {
        if let Some(quotes) = closing {
            if c == '\n' || quotes.contains(&c) {
                closing = None;
            }
            out.push(c);
            rest = &rest[c.len_utf8()..];
        } else if rest.starts_with("//") {
            let n = rest.find('\n').unwrap_or(rest.len());
            out.extend(std::iter::repeat_n(' ', n));
            rest = &rest[n..];
        } else if rest.starts_with("/*") {
            let Some(n) = rest[2..].find("*/").map(|n| n + 4) else {
                let (l, col) = line_col(src, src.len() - rest.len());
                return Err(Diagnostic::new(l, col, "Comment is never closed")
                    .doc("ch02-02-lexical.html#comments").explain("A `/*` block comment runs until `*/`, but there is no `*/` after this point.")
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
            closing = closing_quotes(c);
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_become_spaces_of_the_same_length() {
        for src in [
            "a // é ő\nb /* ű\n∈ */ c",
            "x=\"//\" // y",
            "s='/*' /* ' */ t",
            "„a//b” //c",
            "\"open // not a comment\n// comment",
        ] {
            let clean = strip_comments(src).unwrap();
            assert_eq!(clean.len(), src.len(), "{src}");
            assert_eq!(clean.matches('\n').count(), src.matches('\n').count());
        }
        assert_eq!(strip_comments("a // é\nb").unwrap(), "a      \nb");
        assert_eq!(
            strip_comments("s='/*' /* ' */ t").unwrap(),
            "s='/*'         t"
        );
        assert_eq!(strip_comments("x=\"//\" // y").unwrap(), "x=\"//\"     ");
        assert!(strip_comments("a /* b").is_err());
    }

    #[test]
    fn lexes_every_kind_of_token() {
        let src = "x1 ≤ 3.5 és „szó” <> 'c' .. 12";
        let toks = lex(src, src, 0, src.len()).unwrap();
        let kinds: Vec<&Tok> = toks.iter().map(|t| &t.tok).collect();
        assert_eq!(
            kinds,
            [
                &Tok::Ident("x1".into()),
                &Tok::Sym("<="),
                &Tok::Real(3.5),
                &Tok::Sym("and"),
                &Tok::Str("szó".into()),
                &Tok::Sym("!="),
                &Tok::Str("c".into()),
                &Tok::Sym(".."),
                &Tok::Int(12),
                &Tok::Eof,
            ]
        );
        assert_eq!(toks[4].text, "„szó”");
        assert_eq!(&src[toks[6].start..toks[6].end], "'c'");
        let toks = lex("1..n", "1..n", 0, 4).unwrap();
        assert_eq!(toks[0].tok, Tok::Int(1));
        assert_eq!(toks[1].tok, Tok::Sym(".."));
        assert!(lex("99999999999999999999", "", 0, 20).is_err());
        assert!(lex("\"abc", "\"abc", 0, 4).is_err());
    }
}
