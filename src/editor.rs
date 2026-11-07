//! Code editor widget: syntax highlighting, line numbers and auto-completion.

use eframe::egui::{
    self, Color32, FontId, Id, Key, Modifiers, RichText, TextEdit, TextFormat, text::CCursor,
    text::CCursorRange, text::LayoutJob,
};
use crate::parser::MACROS;

// ---------------------------------------------------------------- colors

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

/// Alt+digit shortcuts for the special characters, in the order of the symbol buttons.
pub const SYMBOL_KEYS: &[(Key, &str)] = &[
    (Key::Num1, "∈"),
    (Key::Num2, "∀"),
    (Key::Num3, "∃"),
    (Key::Num4, "≠"),
    (Key::Num5, "≤"),
    (Key::Num6, "≥"),
];

pub const HEADERS: &[&str] = &[
    "Be", "Sa", "Ki", "Fv", "Ef", "Uf", "In", "Aux", "Out", "Fn", "Pre", "Post", "Def",
];
const KEYWORDS: &[&str] = &[
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
}

impl Job<'_> {
    fn push(&mut self, s: &str, color: Color32) {
        for (i, part) in s.split('\n').enumerate() {
            if i > 0 {
                self.line += 1;
                self.job.append("\n", 0.0, self.fmt(color));
            }
            if !part.is_empty() {
                self.job.append(part, 0.0, self.fmt(color));
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

fn take_while(s: &str, f: impl Fn(char) -> bool) -> usize {
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
        } else if c == '"' {
            let end = rest[1..]
                .find(['"', '\n'])
                .map(|i| i + 2)
                .unwrap_or(rest.len());
            (end.min(rest.len()), p.string)
        } else if c.is_ascii_digit() {
            (
                take_while(rest, |c| c.is_ascii_digit() || c == '.')
                    .min(rest.find("..").unwrap_or(usize::MAX)),
                p.number,
            )
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
                } else if w == "true" || w == "false" {
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

// ---------------------------------------------------------------- completion

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
    for (name, _, _, usage) in MACROS {
        let snippet = match *name {
            "SELECT" => "SELECT(i>=1, $)".to_string(),
            "CONDMAX" | "CONDMIN" => format!("{name}(i=1..n, $, )"),
            _ => format!("{name}(i=1..n, $)"),
        };
        v.push(item(name, &snippet, usage, Macro));
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
        ("mod", "remainder"),
        ("div", "whole-number division"),
        ("true", ""),
        ("false", ""),
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
    }
    v
}

#[derive(Default)]
pub struct Completion {
    owner: Option<Id>,
    items: Vec<Item>,
    sel: usize,
    start: usize, // char index where the typed prefix begins
}

#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    Spec,
    Data,
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

fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map(|(i, _)| i).unwrap_or(s.len())
}

/// Replace chars `a..b` with `insert` (honouring the `$` cursor marker) and move the cursor.
pub fn replace(ctx: &egui::Context, id: Id, text: &mut String, a: usize, b: usize, insert: &str) {
    let cursor_off = insert.find('$').map(|i| insert[..i].chars().count());
    let insert = insert.replace('$', "");
    let (ba, bb) = (char_to_byte(text, a), char_to_byte(text, b));
    text.replace_range(ba..bb, &insert);
    let pos = a + cursor_off.unwrap_or(insert.chars().count());
    set_cursor(ctx, id, pos);
}

pub fn set_cursor(ctx: &egui::Context, id: Id, pos: usize) {
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let mut cur = CCursor::default();
    cur.index.0 = pos;
    state.cursor.set_char_range(Some(CCursorRange::one(cur)));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

pub fn cursor_of(ctx: &egui::Context, id: Id) -> Option<(usize, usize)> {
    let s = egui::text_edit::TextEditState::load(ctx, id)?;
    let r = s.cursor.char_range()?.as_sorted_char_range();
    Some((r.start.0, r.end.0))
}

/// A code editor with line numbers, highlighting and completion.
#[allow(clippy::too_many_arguments)]
pub fn code_editor(
    ui: &mut egui::Ui,
    id: Id,
    text: &mut String,
    lang: Lang,
    symbols: &[Item],
    errors: &[usize],
    comp: &mut Completion,
    hint: &str,
) {
    let dark = ui.visuals().dark_mode;
    let p = palette(dark);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let focused = ui.memory(|m| m.has_focus(id));
    if comp.owner == Some(id) && !focused {
        comp.items.clear();
    }
    let open = comp.owner == Some(id) && !comp.items.is_empty() && focused;

    // keys for the popup must be taken before the TextEdit sees them
    let mut accept: Option<Item> = None;
    if open {
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                comp.sel = (comp.sel + 1) % comp.items.len();
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                comp.sel = (comp.sel + comp.items.len() - 1) % comp.items.len();
            }
            if i.consume_key(Modifiers::NONE, Key::Tab)
                || i.consume_key(Modifiers::NONE, Key::Enter)
            {
                accept = comp.items.get(comp.sel).cloned();
            }
            if i.consume_key(Modifiers::NONE, Key::Escape) {
                comp.items.clear();
            }
        });
    }
    if let Some(it) = accept.take() {
        let cur = cursor_of(ui.ctx(), id).map(|c| c.1).unwrap_or(comp.start);
        replace(ui.ctx(), id, text, comp.start, cur, &it.insert);
        comp.items.clear();
    }

    // Alt+1 … Alt+6 insert ∈ ∀ ∃ ≠ ≤ ≥ (the order of the symbol buttons)
    if focused && lang == Lang::Spec {
        let typed = ui.input_mut(|i| {
            let hit = SYMBOL_KEYS
                .iter()
                .find(|(k, _)| i.consume_key(Modifiers::ALT, *k))
                .map(|(_, s)| *s);
            if hit.is_some() {
                // some platforms also send the key as typed text: the digit, or on macOS
                // the Option-layer character (Option+1 is ¡)
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(t) if t.chars().count() == 1));
            }
            hit
        });
        if let Some(sym) = typed {
            let n = text.chars().count();
            let (a, b) = cursor_of(ui.ctx(), id).unwrap_or((n, n));
            replace(ui.ctx(), id, text, a, b, sym);
            comp.items.clear();
        }
    }

    // Tab inserts two spaces: a real tab character is invalid in YAML test data
    if focused && !open && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
        let n = text.chars().count();
        let (a, b) = cursor_of(ui.ctx(), id).unwrap_or((n, n));
        replace(ui.ctx(), id, text, a, b, "  ");
    }

    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
        let mut job = match lang {
            Lang::Spec => highlight_spec(buf.as_str(), font.clone(), &p, errors),
            Lang::Data => highlight_data(buf.as_str(), font.clone(), &p, errors),
        };
        job.wrap.max_width = f32::INFINITY;
        ui.painter().layout_job(job)
    };

    let lines = text.split('\n').count().max(1);
    let margin = 6.0;
    let out = ui
        .horizontal_top(|ui| {
            // line-number gutter, painted after layout from the galley's own row positions
            let digits = lines.to_string().len().max(2) as f32;
            let gutter_w = digits * font.size * 0.62 + 10.0;
            let (gutter, _) =
                ui.allocate_exact_size(egui::vec2(gutter_w, 0.0), egui::Sense::hover());
            let gutter_x = gutter.right() - 4.0;
            let out = TextEdit::multiline(text)
                .id(id)
                .code_editor()
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::symmetric(6, margin as i8))
                .hint_text(RichText::new(hint).font(font.clone()))
                .desired_width(f32::INFINITY)
                .desired_rows(lines.max(20))
                .layouter(&mut layouter)
                // keep focus on Escape while the popup is open (Escape closes the popup instead)
                .event_filter(egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: open,
                })
                .show(ui);
            let name = if lang == Lang::Spec {
                "Specification editor"
            } else {
                "Test case editor"
            };
            let _ = out.response.response.clone().accessible_name(name);
            let mut line = 1;
            let mut new_line = true;
            for row in &out.galley.rows {
                if new_line {
                    let pos = egui::pos2(gutter_x, out.galley_pos.y + row.pos.y);
                    ui.painter().text(
                        pos,
                        egui::Align2::RIGHT_TOP,
                        line.to_string(),
                        font.clone(),
                        p.gutter,
                    );
                    line += 1;
                }
                new_line = row.ends_with_newline;
            }
            out
        })
        .inner;

    // update the suggestion list after typing
    if out.response.has_focus()
        && let Some(r) = out.cursor_range
    {
        let cur = r.primary.index.0;
        let before: Vec<char> = text.chars().take(cur).collect();
        let plen = before
            .iter()
            .rev()
            .take_while(|c| c.is_alphanumeric() || **c == '_')
            .count();
        let start = cur - plen;
        let prefix: String = before[start..].iter().collect();
        let line_start = before[..start]
            .iter()
            .rev()
            .take_while(|c| **c != '\n')
            .all(|c| c.is_whitespace());
        if out.response.changed()
            && plen > 0
            && !in_comment_or_string(&before[..start], lang)
            && !before
                .get(start.wrapping_sub(1))
                .is_some_and(|c| c.is_ascii_digit())
        {
            let after_elem =
                before[..start].iter().rev().find(|c| !c.is_whitespace()) == Some(&'∈');
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
            items.truncate(12);
            comp.owner = Some(id);
            comp.items = items;
            comp.sel = 0;
            comp.start = start;
        } else if (plen == 0 || start != comp.start) && comp.owner == Some(id) {
            comp.items.clear();
        }
    }
    // popup under the cursor (also on the frame a click moved focus away from the editor)
    if (focused || out.response.has_focus()) && comp.owner == Some(id) && !comp.items.is_empty() {
        let r = out.cursor_range.or_else(|| {
            cursor_of(ui.ctx(), id).map(|c| {
                CCursorRange::one(CCursor {
                    index: egui::text::CharIndex(c.1),
                    ..Default::default()
                })
            })
        });
        if let Some(r) = r {
            let cur = r.primary.index.0;
            {
                let rect = out
                    .galley
                    .pos_from_cursor(r.primary)
                    .translate(out.galley_pos.to_vec2());
                let mut clicked = None;
                egui::Area::new(id.with("completion"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(rect.left_bottom() + egui::vec2(0.0, 2.0))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for (k, it) in comp.items.iter().enumerate() {
                                let color = match it.kind {
                                    Kind::Header => p.header,
                                    Kind::Macro => p.macro_,
                                    Kind::Keyword => p.keyword,
                                    Kind::Var => p.text,
                                    Kind::Type => p.ty,
                                    Kind::Func => p.func,
                                };
                                let mut job = LayoutJob::default();
                                job.append(&it.label, 0.0, TextFormat::simple(font.clone(), color));
                                if !it.detail.is_empty() {
                                    let small = FontId::new(
                                        font.size * 0.8,
                                        egui::FontFamily::Proportional,
                                    );
                                    job.append(
                                        &format!("   {}", it.detail),
                                        0.0,
                                        TextFormat::simple(small, p.comment),
                                    );
                                }
                                let resp = ui.add(
                                    egui::Button::selectable(k == comp.sel, job)
                                        .min_size(egui::vec2(220.0, 0.0)),
                                );
                                if resp.clicked() {
                                    clicked = Some(it.clone());
                                }
                            }
                            ui.label(
                                RichText::new("↑↓ choose · Tab/Enter insert · Esc close")
                                    .small()
                                    .color(p.comment),
                            );
                        });
                    });
                if let Some(it) = clicked {
                    replace(ui.ctx(), id, text, comp.start, cur, &it.insert);
                    comp.items.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    struct S {
        text: String,
        comp: Completion,
        first: bool,
    }

    fn harness(text: &str) -> Harness<'static, S> {
        let id = Id::unique("t-editor");
        let symbols = scan_symbols("Be: szamok∈N[1..n], n∈N\nFv: kisebb:N->L, kisebb(i)=i<3");
        Harness::builder()
            .with_context_setup(crate::app::fonts)
            .build_ui_state(
                move |ui, s: &mut S| {
                    if s.first {
                        s.first = false;
                        let n = s.text.chars().count();
                        set_cursor(ui.ctx(), id, n);
                    }
                    code_editor(
                        ui,
                        id,
                        &mut s.text,
                        Lang::Spec,
                        &symbols,
                        &[],
                        &mut s.comp,
                        "",
                    );
                },
                S {
                    text: text.into(),
                    comp: Completion::default(),
                    first: true,
                },
            )
    }

    #[test]
    fn completes_macro_snippet_with_tab() {
        let mut h = harness("Uf: db=");
        h.run_steps(3);
        h.event(egui::Event::Text("DA".into()));
        h.run_steps(3);
        assert!(
            h.state().comp.items.iter().any(|i| i.label == "DARAB"),
            "{:?}",
            h.state().comp.items
        );
        h.key_press(Key::Tab);
        h.run_steps(3);
        assert_eq!(h.state().text, "Uf: db=DARAB(i=1..n, )");
        assert!(h.state().comp.items.is_empty());
    }

    #[test]
    fn completes_declared_names_and_navigates() {
        let mut h = harness("Uf: x=");
        h.run_steps(3);
        h.event(egui::Event::Text("ki".into()));
        h.run_steps(3);
        let labels: Vec<_> = h
            .state()
            .comp
            .items
            .iter()
            .map(|i| i.label.clone())
            .collect();
        assert_eq!(labels, ["kisebb"]);
        h.key_press(Key::Enter);
        h.run_steps(3);
        assert_eq!(h.state().text, "Uf: x=kisebb()");

        let mut h = harness("Uf: ");
        h.run_steps(3);
        h.event(egui::Event::Text("sz".into()));
        h.run_steps(3);
        h.key_press(Key::Escape);
        h.run_steps(3);
        assert!(h.state().comp.items.is_empty());
        assert_eq!(h.state().text, "Uf: sz");
    }

    #[test]
    fn click_on_suggestion_inserts_it() {
        use egui_kittest::kittest::Queryable;
        let mut h = harness("Uf: x=");
        h.run_steps(3);
        h.event(egui::Event::Text("SE".into()));
        h.run_steps(3);
        h.get_by_label_contains("SEARCH").click();
        h.run_steps(4);
        assert_eq!(h.state().text, "Uf: x=SEARCH(i=1..n, )");
    }

    #[test]
    fn header_and_type_context() {
        let mut h = harness("Be: n∈N\n");
        h.run_steps(3);
        h.event(egui::Event::Text("U".into()));
        h.run_steps(3);
        assert_eq!(
            h.state().comp.items.first().map(|i| i.label.as_str()),
            Some("Uf:")
        );
        let mut h = harness("Be: x∈");
        h.run_steps(3);
        h.event(egui::Event::Text("z".into()));
        h.run_steps(3);
        assert_eq!(
            h.state().comp.items.first().map(|i| i.detail.as_str()),
            Some("integers")
        );
    }

    #[test]
    fn no_completion_in_comments_or_text() {
        for start in ["Uf: x=1 // ", "Uf: /* ", "Uf: x=\""] {
            let mut h = harness(start);
            h.run_steps(3);
            h.event(egui::Event::Text("SU".into()));
            h.run_steps(3);
            assert!(h.state().comp.items.is_empty(), "{start}");
        }
    }

    #[test]
    fn tab_inserts_spaces() {
        let mut h = harness("n:");
        h.run_steps(3);
        h.key_press(Key::Tab);
        h.run_steps(3);
        assert_eq!(h.state().text, "n:  ");
    }

    #[test]
    fn fuzz_highlight_and_scan() {
        let p = palette(false);
        let f = FontId::monospace(14.0);
        let base: Vec<char> = "Be:\nn∈N, t∈Pont[1..n], Pont=(x:Z x y:Z) // é\nFv: f:N->N, f(i)=i\nUf: ∀i∈[1..n]:(t[i].x>0) /* ő */ \"s\" -1.5 a->b\n- name: \"x\" # c\n".chars().collect();
        let mut seed: u64 = 7;
        for _ in 0..3000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let a = (seed >> 33) as usize % base.len();
            let b = (seed >> 13) as usize % base.len();
            let t: String = base[a.min(b)..a.max(b)].iter().collect();
            assert_eq!(highlight_spec(&t, f.clone(), &p, &[2]).text, t);
            assert_eq!(highlight_data(&t, f.clone(), &p, &[2]).text, t);
            let _ = scan_symbols(&t);
        }
    }

    #[test]
    fn alt_digits_insert_symbols() {
        let mut h = harness("Uf: x");
        h.run_steps(3);
        for k in [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
        ] {
            h.key_press_modifiers(Modifiers::ALT, k);
            h.run_steps(2);
        }
        assert_eq!(h.state().text, "Uf: x∈∀∃≠≤≥");
    }

    #[test]
    fn symbol_scan() {
        let s = scan_symbols(
            "Be: n∈N, t∈Pont[1..n], Pont=(x:Z x y:Z)\nFv: f:N->N, f(i)=i\nUf: ∀i∈[1..n]:(t[i].x>0)",
        );
        let l: Vec<_> = s.iter().map(|i| (i.label.as_str(), i.kind)).collect();
        assert_eq!(
            l,
            [
                ("n", Kind::Var),
                ("t", Kind::Var),
                ("Pont", Kind::Type),
                ("f", Kind::Func)
            ]
        );
    }

    #[test]
    fn highlight_does_not_panic_on_odd_input() {
        let p = palette(true);
        let f = FontId::monospace(14.0);
        for t in [
            "",
            "\"unterminated",
            "/* open",
            "1..n",
            "a->b ≤ ∀",
            "Be:\nn∈N\n",
            "é ő ű ∃",
        ] {
            let j = highlight_spec(t, f.clone(), &p, &[1]);
            assert_eq!(j.text, t);
            let j = highlight_data(t, f.clone(), &p, &[1]);
            assert_eq!(j.text, t);
        }
    }
}
