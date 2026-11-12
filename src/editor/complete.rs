//! Auto-completion: what can be typed at the cursor.

use super::Lang;
use super::highlight::{HEADERS, KEYWORDS, take_while};
use crate::parser::{MACROS, macro_alias};
use eframe::egui::Id;

#[derive(Clone, Debug)]
pub struct Item {
    pub label: String,
    /// Text to insert; `$` marks where the cursor ends up.
    pub insert: String,
    pub detail: String,
    pub kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Header,
    Macro,
    Keyword,
    Var,
    Type,
    Func,
}

fn item(label: &str, insert: &str, detail: &str, kind: Kind) -> Item {
    Item {
        label: label.into(),
        insert: insert.into(),
        detail: detail.into(),
        kind,
    }
}

/// Variables, types and functions declared in the (possibly incomplete) specification.
pub fn scan_symbols(src: &str) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut i = 0;
    let mut add = |it: Item| {
        if !out.iter().any(|o| o.label == it.label) {
            out.push(it);
        }
    };
    // text after `pos` up to a top-level comma or the end of the line
    let tail = |pos: usize| -> String {
        let mut depth = 0i32;
        let s = &src[pos..];
        let end = s
            .char_indices()
            .find(|(_, c)| {
                match c {
                    '(' | '[' => depth += 1,
                    ')' | ']' => depth -= 1,
                    _ => {}
                }
                (*c == ',' && depth == 0) || *c == '\n'
            })
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        s[..end].trim().to_string()
    };
    while i < chars.len() {
        let (start, c) = chars[i];
        let prev = if i > 0 { Some(chars[i - 1].1) } else { None };
        if !(c.is_alphabetic() || c == '_') || prev.is_some_and(|p| p.is_alphanumeric() || p == '_')
        {
            i += 1;
            continue;
        }
        let n = take_while(&src[start..], |c| c.is_alphanumeric() || c == '_');
        let w = &src[start..start + n];
        let after = &src[start + n..];
        let after_t = after.trim_start_matches([' ', '\t']);
        let quantified = matches!(prev, Some('∀' | '∃'));
        if !quantified
            && (after_t.starts_with('∈')
                || after_t.starts_with("eleme ")
                || after_t.starts_with("in "))
            && !KEYWORDS.contains(&w)
        {
            let skip = if after_t.starts_with('∈') {
                '∈'.len_utf8()
            } else {
                after_t.find(' ').unwrap() + 1
            };
            let ty_pos = src.len() - after_t.len() + skip;
            add(item(w, w, &format!("∈ {}", tail(ty_pos)), Kind::Var));
        } else if w.starts_with(char::is_uppercase)
            && after_t.starts_with('=')
            && !HEADERS.contains(&w)
        {
            let pos = src.len() - after_t.len() + 1;
            add(item(w, w, &format!("= {}", tail(pos)), Kind::Type));
        } else if after_t.starts_with(':') && !HEADERS.contains(&w) {
            let sig = tail(src.len() - after_t.len() + 1);
            if sig.contains("->") {
                add(item(w, &format!("{w}($)"), &sig, Kind::Func));
            }
        }
        i += w.chars().count();
    }
    out
}

fn spec_items(symbols: &[Item], line_start: bool, after_elem: bool) -> Vec<Item> {
    use Kind::*;
    let mut v = Vec::new();
    if line_start {
        for (h, d) in [
            ("Be", "input variables"),
            ("Sa", "auxiliary variables"),
            ("Ki", "output variables"),
            ("Fv", "functions"),
            ("Ef", "precondition"),
            ("Uf", "postcondition"),
            ("In", "input variables"),
            ("Aux", "auxiliary variables"),
            ("Out", "output variables"),
            ("Fn", "functions"),
            ("Pre", "precondition"),
            ("Post", "postcondition"),
        ] {
            v.push(item(&format!("{h}:"), &format!("{h}:\n$"), d, Header));
        }
    }
    if after_elem {
        for (t, d) in [
            ("N", "natural numbers"),
            ("Z", "integers"),
            ("R", "real numbers"),
            ("L", "logical values"),
            ("S", "texts"),
            ("C", "characters"),
        ] {
            v.push(item(t, t, d, Type));
        }
        v.extend(symbols.iter().filter(|s| s.kind == Type).cloned());
        return v;
    }
    // the Hungarian names of the patterns, after the English ones
    let hu = [
        "SZUMMA",
        "MEGSZÁMOL",
        "ELDÖNT",
        "MIND",
        "KERES",
        "KIVÁLASZT",
        "MÁSOL",
        "KIVÁLOGAT",
        "FELTMAX",
        "FELTMIN",
    ];
    let names = MACROS.iter().map(|m| m.0).chain(hu);
    for name in names {
        let canon = macro_alias(name).unwrap_or(name);
        let Some((.., usage)) = MACROS.iter().find(|m| m.0 == canon) else {
            continue;
        };
        let usage = usage.replace(canon, name);
        let snippet = match canon {
            "SELECT" => format!("{name}(i>=1, $)"),
            "CONDMAX" | "CONDMIN" => format!("{name}(i=1..n, $, )"),
            _ => format!("{name}(i=1..n, $)"),
        };
        v.push(item(name, &snippet, &usage, Macro));
    }
    v.push(item("forall", "∀i∈[1..n]:($)", "∀ for all", Keyword));
    v.push(item("exists", "∃i∈[1..n]:($)", "∃ there exists", Keyword));
    v.push(item("in", "∈", "∈ element of", Keyword));
    v.push(item("neq", "≠", "≠ not equal", Keyword));
    v.push(item("leq", "≤", "≤ less or equal", Keyword));
    v.push(item("geq", "≥", "≥ greater or equal", Keyword));
    for (k, d) in [
        ("and", "logical and"),
        ("or", "logical or"),
        ("not", "negation"),
        ("és", "logical and"),
        ("vagy", "logical or"),
        ("nem", "negation"),
        ("mod", "remainder"),
        ("div", "whole-number division"),
        ("true", ""),
        ("false", ""),
        ("igaz", ""),
        ("hamis", ""),
    ] {
        v.push(item(k, k, d, Keyword));
    }
    for (f, d) in [
        ("length", "number of elements"),
        ("from", "first index"),
        ("to", "last index"),
        ("abs", "absolute value"),
        ("sqrt", "square root"),
        ("round", "nearest whole number"),
        ("hossz", "number of elements"),
        ("tól", "first index"),
        ("ig", "last index"),
        ("kerekít", "nearest whole number"),
    ] {
        v.push(item(f, &format!("{f}($)"), d, Func));
    }
    v.extend(symbols.iter().cloned());
    v
}

fn data_items(symbols: &[Item], line_start: bool) -> Vec<Item> {
    let mut v: Vec<Item> = Vec::new();
    if line_start {
        v.extend(
            symbols
                .iter()
                .filter(|s| s.kind == Kind::Var)
                .map(|s| item(&s.label, &format!("{}: $", s.label), &s.detail, Kind::Var)),
        );
    } else {
        v.push(item("true", "true", "", Kind::Keyword));
        v.push(item("false", "false", "", Kind::Keyword));
        v.push(item("igaz", "igaz", "", Kind::Keyword));
        v.push(item("hamis", "hamis", "", Kind::Keyword));
    }
    v
}

const MAX_ITEMS: usize = 12;

#[derive(Default)]
pub struct Completion {
    pub(super) owner: Option<Id>,
    pub(super) items: Vec<Item>,
    pub(super) sel: usize,
    /// char index where the typed prefix begins
    pub(super) start: usize,
}

impl Completion {
    pub(super) fn is_open_for(&self, id: Id) -> bool {
        self.owner == Some(id) && !self.items.is_empty()
    }

    pub(super) fn update(
        &mut self,
        id: Id,
        text: &str,
        cursor: usize,
        changed: bool,
        lang: Lang,
        symbols: &[Item],
    ) {
        let before: Vec<char> = text.chars().take(cursor).collect();
        let plen = before
            .iter()
            .rev()
            .take_while(|c| c.is_alphanumeric() || **c == '_')
            .count();
        let start = before.len() - plen;
        let (before, prefix) = before.split_at(start);
        let after_digit = before.last().is_some_and(|c| c.is_ascii_digit());
        let suggest = changed && plen > 0 && !after_digit && !in_comment_or_string(before, lang);
        if !suggest {
            if (plen == 0 || start != self.start) && self.owner == Some(id) {
                self.items.clear();
            }
            return;
        }
        let prefix: String = prefix.iter().collect();
        let line_start = before
            .iter()
            .rev()
            .take_while(|c| **c != '\n')
            .all(|c| c.is_whitespace());
        let after_elem = before.iter().rev().find(|c| !c.is_whitespace()) == Some(&'∈');
        let cands = match lang {
            Lang::Spec => spec_items(symbols, line_start, after_elem),
            Lang::Data => data_items(symbols, line_start),
        };
        let lower = prefix.to_lowercase();
        let mut items: Vec<Item> = cands
            .into_iter()
            .filter(|c| {
                c.label.to_lowercase().starts_with(&lower)
                    && !(c.label == prefix && c.insert == prefix)
            })
            .collect();
        // exact-case matches first, keep the rest in their natural order
        items.sort_by_key(|c| !c.label.starts_with(&prefix));
        items.truncate(MAX_ITEMS);
        *self = Completion {
            owner: Some(id),
            items,
            sel: 0,
            start,
        };
    }
}

/// Is the end of `before` inside a comment or a quoted text?
fn in_comment_or_string(before: &[char], lang: Lang) -> bool {
    let text: String = before.iter().collect();
    let line = text.rsplit('\n').next().unwrap_or("");
    let quotes = line.matches('"').count() % 2 == 1;
    match lang {
        Lang::Data => quotes || line.contains('#'),
        Lang::Spec => {
            let open_block = text
                .rfind("/*")
                .is_some_and(|o| text.rfind("*/").is_none_or(|c| c < o));
            quotes || open_block || line.contains("//")
        }
    }
}
