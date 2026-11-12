//! Syntax highlighting for specifications and (YAML) test data.

use crate::parser::{MACROS, closing_quotes};
use eframe::egui::{Color32, FontId, TextFormat, text::LayoutJob};

pub struct Palette {
    pub text: Color32,
    pub header: Color32,
    pub macro_: Color32,
    pub keyword: Color32,
    pub ty: Color32,
    pub number: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub op: Color32,
    pub punct: Color32,
    pub func: Color32,
    pub key: Color32,
    pub error_bg: Color32,
    pub gutter: Color32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            text: Color32::from_rgb(0xd4, 0xd8, 0xe0),
            header: Color32::from_rgb(0xf0, 0x88, 0x3e),
            macro_: Color32::from_rgb(0xc6, 0x8a, 0xf5),
            keyword: Color32::from_rgb(0x5f, 0xb3, 0xf9),
            ty: Color32::from_rgb(0x4e, 0xc9, 0xb0),
            number: Color32::from_rgb(0xe5, 0xc0, 0x7b),
            string: Color32::from_rgb(0x98, 0xc3, 0x79),
            comment: Color32::from_rgb(0x6b, 0x73, 0x80),
            op: Color32::from_rgb(0xff, 0x7a, 0x93),
            punct: Color32::from_rgb(0x8b, 0x93, 0xa1),
            func: Color32::from_rgb(0x61, 0xd0, 0xe8),
            key: Color32::from_rgb(0x5f, 0xb3, 0xf9),
            error_bg: Color32::from_rgba_unmultiplied(0xe5, 0x48, 0x4d, 48),
            gutter: Color32::from_rgb(0x55, 0x5c, 0x68),
        }
    } else {
        Palette {
            text: Color32::from_rgb(0x24, 0x29, 0x2f),
            header: Color32::from_rgb(0xc2, 0x4e, 0x00),
            macro_: Color32::from_rgb(0x82, 0x50, 0xdf),
            keyword: Color32::from_rgb(0x09, 0x69, 0xda),
            ty: Color32::from_rgb(0x0a, 0x7f, 0x6f),
            number: Color32::from_rgb(0x95, 0x5f, 0x00),
            string: Color32::from_rgb(0x1a, 0x7f, 0x37),
            comment: Color32::from_rgb(0x8c, 0x95, 0x9f),
            op: Color32::from_rgb(0xcf, 0x22, 0x2e),
            punct: Color32::from_rgb(0x57, 0x60, 0x6a),
            func: Color32::from_rgb(0x05, 0x7f, 0xa5),
            key: Color32::from_rgb(0x09, 0x69, 0xda),
            error_bg: Color32::from_rgba_unmultiplied(0xff, 0x81, 0x82, 70),
            gutter: Color32::from_rgb(0xa0, 0xa7, 0xb0),
        }
    }
}

pub const HEADERS: &[&str] = &[
    "Be", "Sa", "Ki", "Fv", "Ef", "Uf", "In", "Aux", "Out", "Fn", "Pre", "Post", "Def",
];
pub(super) const KEYWORDS: &[&str] = &[
    "and", "or", "not", "és", "vagy", "nem", "mod", "div", "eleme", "in", "exists", "forall",
    "létezik", "minden", "true", "false", "igaz", "hamis",
];
const GLYPHS: &str = "∈∀∃≠≤≥∤¬–−";

// ---------------------------------------------------------------- highlighting

/// Appends text segments, tinting lines listed in `errors` (1-based).
struct Job<'a> {
    job: LayoutJob,
    font: FontId,
    line: usize,
    errors: &'a [usize],
    err_bg: Color32,
    last: Color32,
}

impl Job<'_> {
    fn push(&mut self, s: &str, color: Color32) {
        for (i, part) in s.split('\n').enumerate() {
            if i > 0 {
                self.line += 1;
                self.job.append("\n", 0.0, self.fmt(color));
            }
            if !part.is_empty() {
                let blank = part.bytes().all(|b| b == b' ' || b == b'\t');
                let color = if blank { self.last } else { color };
                self.job.append(part, 0.0, self.fmt(color));
                self.last = color;
            }
        }
    }
    fn fmt(&self, color: Color32) -> TextFormat {
        let mut f = TextFormat::simple(self.font.clone(), color);
        if self.errors.contains(&self.line) {
            f.background = self.err_bg;
        }
        f
    }
}

pub(super) fn take_while(s: &str, f: impl Fn(char) -> bool) -> usize {
    s.char_indices()
        .find(|(_, c)| !f(*c))
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

pub fn highlight_spec(text: &str, font: FontId, p: &Palette, errors: &[usize]) -> LayoutJob {
    let mut j = Job {
        job: LayoutJob::default(),
        font,
        line: 1,
        errors,
        err_bg: p.error_bg,
        last: p.text,
    };
    let mut rest = text;
    let mut line_start = true;
    while let Some(c) = rest.chars().next() {
        let (n, color) = if rest.starts_with("//") {
            (rest.find('\n').unwrap_or(rest.len()), p.comment)
        } else if rest.starts_with("/*") {
            (
                rest.split_at(2)
                    .1
                    .find("*/")
                    .map(|i| i + 4)
                    .unwrap_or(rest.len()),
                p.comment,
            )
        } else if let Some(close) = closing_quotes(c) {
            let body = &rest[c.len_utf8()..];
            let len = match body
                .char_indices()
                .find(|(_, d)| close.contains(d) || *d == '\n')
            {
                Some((i, d)) if d != '\n' => i + d.len_utf8(),
                Some((i, _)) => i,
                None => body.len(),
            };
            (c.len_utf8() + len, p.string)
        } else if c.is_ascii_digit() {
            let n = take_while(rest, |c| c.is_ascii_digit() || c == '.');
            (rest[..n].find("..").unwrap_or(n), p.number)
        } else if c.is_alphabetic() || c == '_' {
            let n = take_while(rest, |c| c.is_alphanumeric() || c == '_');
            let w = &rest[..n];
            let after = &rest[n..];
            if line_start && HEADERS.contains(&w) && after.starts_with(':') {
                (n + 1, p.header)
            } else if MACROS.iter().any(|m| m.0 == w) {
                (n, p.macro_)
            } else if KEYWORDS.contains(&w) {
                (n, p.keyword)
            } else if w.starts_with(char::is_uppercase) {
                (n, p.ty)
            } else if after.starts_with('(') {
                (n, p.func)
            } else {
                (n, p.text)
            }
        } else if GLYPHS.contains(c) || ["->", "<=", ">=", "!="].iter().any(|o| rest.starts_with(o))
        {
            let n = if c.is_ascii() { 2 } else { c.len_utf8() };
            (n, p.op)
        } else if c.is_whitespace() {
            (c.len_utf8(), p.text)
        } else {
            (c.len_utf8(), p.punct)
        };
        let n = n.max(c.len_utf8());
        let seg = &rest[..n];
        j.push(seg, color);
        if !c.is_whitespace() {
            line_start = false;
        }
        if seg.ends_with('\n') || c == '\n' {
            line_start = true;
        }
        rest = &rest[n..];
    }
    j.job
}

pub fn highlight_data(text: &str, font: FontId, p: &Palette, errors: &[usize]) -> LayoutJob {
    let mut j = Job {
        job: LayoutJob::default(),
        font,
        line: 1,
        errors,
        err_bg: p.error_bg,
        last: p.text,
    };
    for (li, line) in text.split('\n').enumerate() {
        if li > 0 {
            j.push("\n", p.text);
        }
        let mut rest = line;
        // optional list dash and a `key:`
        let ws = take_while(rest, char::is_whitespace);
        j.push(&rest[..ws], p.text);
        rest = &rest[ws..];
        if let Some(r) = rest.strip_prefix("- ") {
            j.push("- ", p.punct);
            rest = r;
        }
        let k = take_while(rest, |c| c.is_alphanumeric() || c == '_');
        if k > 0 && rest[k..].starts_with(':') {
            j.push(&rest[..k], p.key);
            j.push(":", p.punct);
            rest = &rest[k + 1..];
        }
        while let Some(c) = rest.chars().next() {
            let (n, color) = if c == '#' {
                (rest.len(), p.comment)
            } else if c == '"' || c == '\'' {
                (
                    rest[1..].find(c).map(|i| i + 2).unwrap_or(rest.len()),
                    p.string,
                )
            } else if c.is_ascii_digit()
                || (c == '-' && rest[1..].starts_with(|d: char| d.is_ascii_digit()))
            {
                (
                    1 + take_while(&rest[1..], |c| c.is_ascii_digit() || c == '.'),
                    p.number,
                )
            } else if c.is_alphanumeric() || c == '_' {
                let n = take_while(rest, |c| c.is_alphanumeric() || c == '_');
                let w = &rest[..n];
                if rest[n..].trim_start().starts_with(':') {
                    (n, p.key)
                } else if matches!(w, "true" | "false" | "igaz" | "hamis") {
                    (n, p.keyword)
                } else {
                    (n, p.string)
                }
            } else if c.is_whitespace() {
                (c.len_utf8(), p.text)
            } else {
                (c.len_utf8(), p.punct)
            };
            j.push(&rest[..n], color);
            rest = &rest[n..];
        }
    }
    j.job
}
