//! Vim keybindings for the code editor: Normal, Insert, Visual and Visual-line modes.
//!
//! The keys are taken from the input before the `TextEdit` sees them. Insert mode leaves
//! everything to the `TextEdit` except Escape; the other modes run the keys as commands
//! on the text and the cursor.

use std::collections::HashMap;

use eframe::egui::{self, Event, Id, Key, Modifiers, text::CCursor, text::CCursorRange};

use super::{cursor_of, set_cursor};

const ESC: char = '\x1b';
const ENTER: char = '\r';
const BACK: char = '\x08';
const CTRL_R: char = '\x12';

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual => "VISUAL",
            Mode::VisualLine => "V-LINE",
        }
    }

    fn visual(self) -> bool {
        matches!(self, Mode::Visual | Mode::VisualLine)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// The range ends before the target.
    Excl,
    /// The range includes the target character.
    Incl,
    /// Whole lines.
    Line,
}

enum Mo {
    /// More keys are needed.
    More,
    /// Not a motion (or it can't move): the command is dropped.
    Bad,
    To(usize, Kind),
}

#[derive(Default)]
struct History {
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
}

/// The last change, for `.`: the command's keys and the text typed in Insert mode after it.
#[derive(Clone, Default)]
struct Change {
    keys: Vec<char>,
    text: String,
}

/// The text being edited, as chars, with the cursor.
struct Ed {
    t: Vec<char>,
    p: usize,
    changed: bool,
}

impl Ed {
    fn new(text: &str, p: usize) -> Ed {
        Ed {
            t: text.chars().collect(),
            p,
            changed: false,
        }
    }

    fn splice(&mut self, s: usize, e: usize, with: &[char]) {
        self.t.splice(s..e, with.iter().copied());
        self.changed = true;
    }
}

#[derive(Default)]
pub struct Vim {
    pub mode: Mode,
    /// Keys of the command typed so far.
    keys: Vec<char>,
    reg: String,
    reg_line: bool,
    /// Last `f`/`t`/`F`/`T`: (command, target).
    find: Option<(char, char)>,
    /// Last `*`/`#` search: (word, forward).
    search: Option<(Vec<char>, bool)>,
    /// Column kept by `j`/`k` across shorter lines.
    want: Option<usize>,
    keep_want: bool,
    /// Visual mode: where the selection started, and the cursor.
    anchor: usize,
    cur: usize,
    /// Visual mode: the cursor range last written, to notice mouse clicks.
    shown: Option<(usize, usize)>,
    hist: HashMap<Id, History>,
    id: Option<Id>,
    /// An undo step was saved for the command being run.
    saved: bool,
    last: Option<Change>,
    /// Recording the text typed in Insert mode for `.`: (start, text length at start).
    typing: Option<(usize, usize)>,
    replaying: bool,
}

impl Vim {
    /// Text for the status bar: the mode and a half-typed command.
    pub fn status(&self) -> String {
        let keys: String = self.keys.iter().filter(|c| !c.is_control()).collect();
        if keys.is_empty() {
            self.mode.label().into()
        } else {
            format!("{}  {keys}", self.mode.label())
        }
    }

    pub fn reset(&mut self) {
        self.mode = Mode::Normal;
        self.keys.clear();
        self.typing = None;
        self.shown = None;
    }

    pub fn inserting(&self) -> bool {
        self.mode == Mode::Insert
    }

    /// Where to paint the block cursor (Visual modes; Normal mode uses the text cursor).
    pub fn visual_cursor(&self) -> Option<usize> {
        self.mode.visual().then_some(self.cur)
    }

    /// Run the keys of this frame. Call before the `TextEdit` is shown, while it has focus.
    pub fn handle(&mut self, ctx: &egui::Context, id: Id, text: &mut String) {
        if self.id != Some(id) {
            // another editor got the focus
            if self.mode.visual() {
                self.mode = Mode::Normal;
            }
            self.keys.clear();
            self.id = Some(id);
        }
        let n = text.chars().count();
        let (a, b) = cursor_of(ctx, id).unwrap_or((n, n));
        if self.mode == Mode::Insert {
            let esc = ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Escape)
                    || i.consume_key(Modifiers::CTRL, Key::OpenBracket)
            });
            if esc {
                let mut ed = Ed::new(text, b);
                self.leave_insert(&mut ed);
                set_cursor(ctx, id, clamp_normal(&ed.t, ed.p));
            }
            return;
        }
        if self.mode.visual() && self.shown != Some((a, b)) {
            // the mouse moved the cursor
            self.mode = Mode::Normal;
        }
        let keys = take_keys(ctx);
        if !self.mode.visual() && a != b && keys.is_empty() {
            // a selection made with the mouse: leave it alone until a key comes
            return;
        }
        let p = if self.mode.visual() { self.cur } else { b };
        let mut ed = Ed::new(text, p);
        for k in keys {
            self.press(&mut ed, k);
        }
        if ed.changed {
            *text = ed.t.iter().collect();
        }
        self.show(ctx, id, &mut ed);
    }

    /// Write the cursor (or the visual selection) back to the `TextEdit`.
    fn show(&mut self, ctx: &egui::Context, id: Id, ed: &mut Ed) {
        let t = &ed.t;
        ed.p = ed.p.min(t.len());
        self.shown = None;
        match self.mode {
            Mode::Insert => set_cursor(ctx, id, ed.p),
            Mode::Normal => set_cursor(ctx, id, clamp_normal(t, ed.p)),
            Mode::Visual | Mode::VisualLine => {
                self.anchor = self.anchor.min(t.len());
                self.cur = ed.p;
                let (s, e) = self.selection(t, ed.p);
                let (pri, sec) = if ed.p >= self.anchor { (e, s) } else { (s, e) };
                set_range(ctx, id, pri, sec);
                self.shown = Some((s, e));
            }
        }
    }

    /// The selected chars `s..e` in a Visual mode.
    fn selection(&self, t: &[char], p: usize) -> (usize, usize) {
        let (s, e) = (self.anchor.min(p), self.anchor.max(p));
        if self.mode == Mode::VisualLine {
            (ls(t, s), le(t, e))
        } else {
            (s, (e + 1).min(t.len()))
        }
    }

    fn press(&mut self, ed: &mut Ed, k: char) {
        if k == ESC && !self.keys.is_empty() {
            // cancels a half-typed command
            self.keys.clear();
            return;
        }
        self.keys.push(k);
        let keys = self.keys.clone();
        self.saved = false;
        self.keep_want = false;
        let was_changed = std::mem::take(&mut ed.changed);
        let before = self.mode;
        let done = if self.mode.visual() {
            self.visual(ed, &keys)
        } else {
            self.normal(ed, &keys)
        };
        let changed = ed.changed;
        ed.changed |= was_changed;
        if !done {
            return;
        }
        self.keys.clear();
        if !self.keep_want {
            self.want = None;
        }
        // remember the change for `.`
        let undoing = matches!(keys.last(), Some(&('u' | CTRL_R | '.')));
        if before == Mode::Normal && !self.replaying && !undoing {
            if self.mode == Mode::Insert {
                self.last = Some(Change {
                    keys,
                    text: String::new(),
                });
                self.typing = Some((ed.p, ed.t.len()));
            } else if changed {
                self.last = Some(Change {
                    keys,
                    text: String::new(),
                });
            }
        }
    }

    fn save(&mut self, ed: &Ed) {
        if self.saved {
            return;
        }
        self.saved = true;
        let h = self.hist.entry(self.id.unwrap_or(Id::NULL)).or_default();
        h.undo.push((ed.t.iter().collect(), ed.p));
        if h.undo.len() > 500 {
            h.undo.remove(0);
        }
        h.redo.clear();
    }

    fn undo(&mut self, ed: &mut Ed, n: usize, redo: bool) {
        let h = self.hist.entry(self.id.unwrap_or(Id::NULL)).or_default();
        for _ in 0..n {
            let now: String = ed.t.iter().collect();
            let (from, to) = if redo {
                (&mut h.redo, &mut h.undo)
            } else {
                (&mut h.undo, &mut h.redo)
            };
            // skip steps that didn't change anything
            while from.last().is_some_and(|(s, _)| *s == now) {
                from.pop();
            }
            let Some((s, p)) = from.pop() else { break };
            to.push((now, ed.p));
            ed.t = s.chars().collect();
            ed.p = p;
            ed.changed = true;
        }
    }

    fn start_insert(&mut self, ed: &Ed) {
        self.save(ed);
        self.mode = Mode::Insert;
    }

    fn leave_insert(&mut self, ed: &mut Ed) {
        self.mode = Mode::Normal;
        if let Some((start, len0)) = self.typing.take()
            && let Some(last) = &mut self.last
            && ed.p >= start
            && ed.t.len() >= len0
            && ed.t.len() - len0 == ed.p - start
        {
            last.text = ed.t[start..ed.p].iter().collect();
        }
        if ed.p > ls(&ed.t, ed.p) {
            ed.p -= 1;
        }
        // entering and leaving Insert mode without typing is no undo step
        let h = self.hist.entry(self.id.unwrap_or(Id::NULL)).or_default();
        let now: String = ed.t.iter().collect();
        if h.undo.last().is_some_and(|(s, _)| *s == now) {
            h.undo.pop();
        }
    }

    fn yank(&mut self, t: &[char], s: usize, e: usize, line: bool) {
        self.reg = t[s..e].iter().collect();
        if line {
            self.reg.push('\n');
        }
        self.reg_line = line;
    }

    /// Run an operator (`d`, `c`, `y`, `>`, `<`, `~`, `u`, `U`) from `a` to `b`.
    fn apply(&mut self, ed: &mut Ed, op: char, a: usize, b: usize, kind: Kind) {
        let len = ed.t.len();
        let (mut s, mut e) = (a.min(b).min(len), a.max(b).min(len));
        match kind {
            Kind::Excl => {}
            Kind::Incl => e = (e + 1).min(len),
            Kind::Line => {
                s = ls(&ed.t, s);
                e = le(&ed.t, e);
            }
        }
        let line = kind == Kind::Line;
        match op {
            'y' => {
                self.yank(&ed.t, s, e, line);
                ed.p = a.min(b);
            }
            'd' => {
                self.save(ed);
                self.yank(&ed.t, s, e, line);
                if line {
                    // take one line break along
                    let (ds, de) = if e < len {
                        (s, e + 1)
                    } else {
                        (s.saturating_sub(1), e)
                    };
                    ed.splice(ds, de, &[]);
                    let at = ls(&ed.t, ds.min(ed.t.len()));
                    ed.p = fnb(&ed.t, at);
                } else {
                    ed.splice(s, e, &[]);
                    ed.p = s;
                }
            }
            'c' => {
                self.save(ed);
                self.yank(&ed.t, s, e, line);
                if line {
                    let indent = indent_of(&ed.t, s);
                    ed.splice(s, e, &indent);
                    ed.p = s + indent.len();
                } else {
                    ed.splice(s, e, &[]);
                    ed.p = s;
                }
                self.mode = Mode::Insert;
            }
            '>' | '<' => {
                self.save(ed);
                let first = line_no(&ed.t, s);
                let last = line_no(&ed.t, if line || e == s { e } else { e - 1 });
                for n in first..=last {
                    let st = line_start_n(&ed.t, n);
                    let en = le(&ed.t, st);
                    if op == '>' {
                        if en > st {
                            ed.splice(st, st, &[' ', ' ']);
                        }
                    } else {
                        let drop = if ed.t.get(st) == Some(&'\t') {
                            1
                        } else {
                            ed.t[st..en]
                                .iter()
                                .take(2)
                                .take_while(|c| **c == ' ')
                                .count()
                        };
                        ed.splice(st, st + drop, &[]);
                    }
                }
                ed.p = fnb(&ed.t, line_start_n(&ed.t, first));
            }
            '~' | 'u' | 'U' => {
                self.save(ed);
                let new: Vec<char> = ed.t[s..e]
                    .iter()
                    .flat_map(|&c| match op {
                        'u' => c.to_lowercase().collect::<Vec<_>>(),
                        'U' => c.to_uppercase().collect(),
                        _ if c.is_uppercase() => c.to_lowercase().collect(),
                        _ => c.to_uppercase().collect(),
                    })
                    .collect();
                ed.splice(s, e, &new);
                ed.p = s;
            }
            _ => {}
        }
    }

    fn put(&mut self, ed: &mut Ed, before: bool, n: usize) {
        if self.reg.is_empty() {
            return;
        }
        self.save(ed);
        let txt: Vec<char> = self.reg.repeat(n).chars().collect();
        if self.reg_line {
            if before {
                let at = ls(&ed.t, ed.p);
                ed.splice(at, at, &txt);
                ed.p = fnb(&ed.t, at);
            } else {
                let l = le(&ed.t, ed.p);
                if l < ed.t.len() {
                    ed.splice(l + 1, l + 1, &txt);
                } else {
                    // the last line has no line break: move it to the front
                    let mut v = vec!['\n'];
                    v.extend_from_slice(&txt[..txt.len() - 1]);
                    ed.splice(l, l, &v);
                }
                ed.p = fnb(&ed.t, l + 1);
            }
        } else {
            let at = if before || ed.p >= ed.t.len() || ed.t[ed.p] == '\n' {
                ed.p
            } else {
                ed.p + 1
            };
            ed.splice(at, at, &txt);
            ed.p = at + txt.len() - 1;
        }
    }

    /// Join `n` lines (at least two) from the cursor's.
    fn join(&mut self, ed: &mut Ed, n: usize) {
        self.save(ed);
        for _ in 0..n.max(2) - 1 {
            let l = le(&ed.t, ed.p);
            if l >= ed.t.len() {
                break;
            }
            let mut e = l + 1;
            while e < ed.t.len() && matches!(ed.t[e], ' ' | '\t') {
                e += 1;
            }
            let next_empty = e >= ed.t.len() || ed.t[e] == '\n' || ed.t[e] == ')';
            let ends_blank = l == 0 || matches!(ed.t[l - 1], ' ' | '\t' | '\n');
            let sep: &[char] = if next_empty || ends_blank {
                &[]
            } else {
                &[' ']
            };
            ed.splice(l, e, sep);
            ed.p = l;
        }
    }

    /// Normal mode: returns false while the command needs more keys.
    fn normal(&mut self, ed: &mut Ed, keys: &[char]) -> bool {
        let mut i = 0;
        let n1 = count(keys, &mut i);
        let Some(&c) = keys.get(i) else {
            return false;
        };
        let rest = &keys[i + 1..];
        let n = n1.unwrap_or(1);
        let len = ed.t.len();
        let p = ed.p.min(len);
        let (l0, l1) = (ls(&ed.t, p), le(&ed.t, p));
        // shorthands like `D` = `d$`, keeping the count
        let alias = |s: &str| {
            let mut v: Vec<char> = n1
                .map(|n| n.to_string().chars().collect())
                .unwrap_or_default();
            v.extend(s.chars());
            v.extend_from_slice(rest);
            v
        };
        match c {
            ESC => {}
            'i' => self.start_insert(ed),
            'a' => {
                if p < len && ed.t[p] != '\n' {
                    ed.p = p + 1;
                }
                self.start_insert(ed);
            }
            'I' => {
                ed.p = fnb(&ed.t, l0);
                self.start_insert(ed);
            }
            'A' => {
                ed.p = l1;
                self.start_insert(ed);
            }
            'o' | 'O' => {
                self.save(ed);
                let indent = indent_of(&ed.t, l0);
                if c == 'o' {
                    let mut v = vec!['\n'];
                    v.extend_from_slice(&indent);
                    ed.splice(l1, l1, &v);
                    ed.p = l1 + 1 + indent.len();
                } else {
                    let mut v = indent.clone();
                    v.push('\n');
                    ed.splice(l0, l0, &v);
                    ed.p = l0 + indent.len();
                }
                self.start_insert(ed);
            }
            'x' if p < l1 => self.apply(ed, 'd', p, (p + n).min(l1), Kind::Excl),
            'X' if p > l0 => self.apply(ed, 'd', p.saturating_sub(n).max(l0), p, Kind::Excl),
            'x' | 'X' => {}
            's' => {
                self.apply(ed, 'c', p, (p + n).min(l1), Kind::Excl);
            }
            'S' => return self.normal(ed, &alias("cc")),
            'C' => return self.normal(ed, &alias("c$")),
            'D' => return self.normal(ed, &alias("d$")),
            'Y' => return self.normal(ed, &alias("yy")),
            'r' => {
                let Some(&ch) = rest.first() else {
                    return false;
                };
                if ch != ESC && p + n <= l1 {
                    self.save(ed);
                    if ch == ENTER {
                        ed.splice(p, p + n, &['\n']);
                        ed.p = p + 1;
                    } else {
                        ed.splice(p, p + n, &vec![ch; n]);
                        ed.p = p + n - 1;
                    }
                }
            }
            'J' => self.join(ed, n),
            '~' => {
                if p < l1 {
                    let e = (p + n).min(l1);
                    self.apply(ed, '~', p, e, Kind::Excl);
                    ed.p = e;
                }
            }
            'p' | 'P' => self.put(ed, c == 'P', n),
            'u' => self.undo(ed, n, false),
            CTRL_R => self.undo(ed, n, true),
            '.' => {
                if let Some(ch) = self.last.clone() {
                    let mut keys = ch.keys.clone();
                    if let Some(n) = n1 {
                        let mut j = 0;
                        count(&keys, &mut j);
                        keys = n
                            .to_string()
                            .chars()
                            .chain(keys[j..].iter().copied())
                            .collect();
                    }
                    self.replaying = true;
                    self.normal(ed, &keys);
                    if self.mode == Mode::Insert {
                        let txt: Vec<char> = ch.text.chars().collect();
                        ed.splice(ed.p, ed.p, &txt);
                        ed.p += txt.len();
                        self.leave_insert(ed);
                    }
                    self.replaying = false;
                }
            }
            'v' | 'V' => {
                self.mode = if c == 'v' {
                    Mode::Visual
                } else {
                    Mode::VisualLine
                };
                self.anchor = p;
            }
            'd' | 'c' | 'y' | '>' | '<' => return self.operator(ed, c, n1, rest),
            'g' if rest.first() == Some(&'~') => return self.operator(ed, '~', n1, &rest[1..]),
            'g' if rest.first() == Some(&'u') => return self.operator(ed, 'u', n1, &rest[1..]),
            'g' if rest.first() == Some(&'U') => return self.operator(ed, 'U', n1, &rest[1..]),
            '*' | '#' => {
                if let Some((s, e)) = word_at(&ed.t, p) {
                    let word = ed.t[s..e].to_vec();
                    if let Some(q) = search(&ed.t, s, &word, c == '*', n) {
                        ed.p = q;
                    }
                    self.search = Some((word, c == '*'));
                }
            }
            'n' | 'N' => {
                if let Some((word, fwd)) = &self.search
                    && let Some(q) = search(&ed.t, p, word, *fwd == (c == 'n'), n)
                {
                    ed.p = q;
                }
            }
            _ => match self.motion(&ed.t, p, &keys[i..], n, n1.is_some(), None) {
                Mo::More => return false,
                Mo::Bad => {}
                Mo::To(q, _) => ed.p = q,
            },
        }
        true
    }

    /// `d`, `c`, `y`, `>`, `<`, `g~`, `gu`, `gU` followed by a motion or a text object.
    fn operator(&mut self, ed: &mut Ed, op: char, n1: Option<usize>, rest: &[char]) -> bool {
        let mut j = 0;
        let n2 = count(rest, &mut j);
        let Some(&m) = rest.get(j) else {
            return false;
        };
        let counted = n1.is_some() || n2.is_some();
        let n = n1.unwrap_or(1) * n2.unwrap_or(1);
        let t = &ed.t;
        let p = ed.p.min(t.len());
        if m == ESC {
            return true;
        }
        // `dd`, `cc`, `yy`, `>>`, `<<`, `g~~`: whole lines
        if m == op {
            let last = line_start_n(t, line_no(t, p) + n - 1);
            self.apply(ed, op, p, last, Kind::Line);
            return true;
        }
        if m == 'i' || m == 'a' {
            let Some(&o) = rest.get(j + 1) else {
                return false;
            };
            if let Some((s, e)) = text_object(t, p, m == 'a', o)
                && e > s
            {
                self.apply(ed, op, s, e, Kind::Excl);
            }
            return true;
        }
        // `cw` changes to the end of the word, like `ce`
        if op == 'c' && (m == 'w' || m == 'W') && p < t.len() && cls(t[p], m == 'W') != 0 {
            let big = m == 'W';
            let at_end = p + 1 >= t.len() || cls(t[p + 1], big) != cls(t[p], big);
            let mut q = p;
            for _ in 0..n - usize::from(at_end) {
                q = word_end(t, q, big);
            }
            self.apply(ed, op, p, q, Kind::Incl);
            return true;
        }
        match self.motion(t, p, &rest[j..], n, counted, Some(op)) {
            Mo::More => return false,
            Mo::Bad => {}
            Mo::To(q, kind) => self.apply(ed, op, p, q, kind),
        }
        true
    }

    /// Visual and Visual-line modes.
    fn visual(&mut self, ed: &mut Ed, keys: &[char]) -> bool {
        let mut i = 0;
        let n1 = count(keys, &mut i);
        let Some(&c) = keys.get(i) else {
            return false;
        };
        let n = n1.unwrap_or(1);
        let p = ed.p.min(ed.t.len());
        let kind = if self.mode == Mode::VisualLine {
            Kind::Line
        } else {
            Kind::Incl
        };
        let op = match c {
            'd' | 'x' => Some(('d', kind)),
            'X' | 'D' => Some(('d', Kind::Line)),
            'y' => Some(('y', kind)),
            'Y' => Some(('y', Kind::Line)),
            'c' | 's' => Some(('c', kind)),
            'C' | 'S' | 'R' => Some(('c', Kind::Line)),
            '>' | '<' | '~' | 'u' | 'U' => Some((c, kind)),
            _ => None,
        };
        if let Some((op, kind)) = op {
            let a = self.anchor;
            self.apply(ed, op, a, p, kind);
            if self.mode != Mode::Insert {
                self.mode = Mode::Normal;
            }
            return true;
        }
        match c {
            ESC => self.mode = Mode::Normal,
            'v' | 'V' => {
                let m = if c == 'v' {
                    Mode::Visual
                } else {
                    Mode::VisualLine
                };
                self.mode = if self.mode == m { Mode::Normal } else { m };
            }
            'o' => {
                ed.p = self.anchor;
                self.anchor = p;
            }
            'J' => {
                let lines = line_no(&ed.t, self.anchor.max(p)) - line_no(&ed.t, self.anchor.min(p));
                ed.p = self.anchor.min(p);
                self.join(ed, lines + 1);
                self.mode = Mode::Normal;
            }
            'p' | 'P' => {
                let (reg, line) = (self.reg.clone(), self.reg_line);
                let a = self.anchor;
                self.apply(ed, 'd', a, p, kind);
                self.saved = true;
                (self.reg, self.reg_line) = (reg, line);
                let before = !(line && kind == Kind::Line && ed.p >= ed.t.len());
                self.put(ed, before, 1);
                self.mode = Mode::Normal;
            }
            'r' => {
                let Some(&ch) = keys.get(i + 1) else {
                    return false;
                };
                if ch != ESC {
                    self.save(ed);
                    let (s, e) = self.selection(&ed.t, p);
                    for k in s..e {
                        if ed.t[k] != '\n' {
                            ed.t[k] = ch;
                        }
                    }
                    ed.changed = true;
                    ed.p = s;
                }
                self.mode = Mode::Normal;
            }
            'i' | 'a' => {
                let Some(&o) = keys.get(i + 1) else {
                    return false;
                };
                if let Some((s, e)) = text_object(&ed.t, p, c == 'a', o)
                    && e > s
                {
                    self.anchor = s;
                    ed.p = e - 1;
                }
            }
            _ => match self.motion(&ed.t, p, &keys[i..], n, n1.is_some(), None) {
                Mo::More => return false,
                Mo::Bad => {}
                Mo::To(q, _) => ed.p = q,
            },
        }
        true
    }

    fn motion(
        &mut self,
        t: &[char],
        p: usize,
        keys: &[char],
        n: usize,
        counted: bool,
        op: Option<char>,
    ) -> Mo {
        let Some(&m) = keys.first() else {
            return Mo::More;
        };
        let (l0, l1) = (ls(t, p), le(t, p));
        match m {
            'h' | BACK => Mo::To(p.saturating_sub(n).max(l0), Kind::Excl),
            'l' | ' ' => Mo::To((p + n).min(l1), Kind::Excl),
            'j' | 'k' | '+' | '-' | ENTER | '_' => {
                let line = line_no(t, p);
                let target = match m {
                    'j' | '+' | ENTER => (line + n).min(nlines(t) - 1),
                    '_' => (line + n - 1).min(nlines(t) - 1),
                    _ => line.saturating_sub(n),
                };
                let s = line_start_n(t, target);
                let q = if matches!(m, 'j' | 'k') {
                    let col = self.want.unwrap_or(p - l0);
                    self.want = Some(col);
                    self.keep_want = true;
                    (s + col).min(le(t, s))
                } else {
                    fnb(t, s)
                };
                Mo::To(q, Kind::Line)
            }
            '0' => Mo::To(l0, Kind::Excl),
            '^' => Mo::To(fnb(t, l0), Kind::Excl),
            '$' => {
                let s = line_start_n(t, line_no(t, p) + n - 1);
                let e = le(t, s);
                if e > s {
                    Mo::To(e - 1, Kind::Incl)
                } else {
                    Mo::To(e, Kind::Excl)
                }
            }
            'w' | 'W' => {
                let big = m == 'W';
                let mut q = p;
                for _ in 0..n {
                    let prev = q;
                    q = word_fwd(t, q, big);
                    // `dw` on the last word of a line stops at the line break
                    if op.is_some() && line_no(t, q) > line_no(t, prev) && le(t, prev) > prev {
                        q = le(t, prev);
                        break;
                    }
                }
                Mo::To(q, Kind::Excl)
            }
            'e' | 'E' => {
                let mut q = p;
                for _ in 0..n {
                    q = word_end(t, q, m == 'E');
                }
                Mo::To(q, Kind::Incl)
            }
            'b' | 'B' => {
                let mut q = p;
                for _ in 0..n {
                    q = word_back(t, q, m == 'B');
                }
                Mo::To(q, Kind::Excl)
            }
            'G' => {
                let line = if counted { n - 1 } else { nlines(t) - 1 };
                Mo::To(fnb(t, line_start_n(t, line)), Kind::Line)
            }
            'g' => match keys.get(1) {
                None => Mo::More,
                Some('g') => {
                    let line = if counted { n - 1 } else { 0 };
                    Mo::To(fnb(t, line_start_n(t, line)), Kind::Line)
                }
                Some('e') => {
                    let mut q = p;
                    for _ in 0..n {
                        q = word_end_back(t, q, false);
                    }
                    Mo::To(q, Kind::Incl)
                }
                Some('_') => {
                    let s = line_start_n(t, line_no(t, p) + n - 1);
                    let mut e = le(t, s);
                    while e > s && matches!(t[e - 1], ' ' | '\t') {
                        e -= 1;
                    }
                    Mo::To(e.saturating_sub(1).max(s), Kind::Incl)
                }
                _ => Mo::Bad,
            },
            '}' | '{' => {
                let mut q = p;
                for _ in 0..n {
                    q = paragraph(t, q, m == '}');
                }
                Mo::To(q, Kind::Excl)
            }
            'f' | 't' | 'F' | 'T' => match keys.get(1) {
                None => Mo::More,
                Some(&ESC) => Mo::Bad,
                Some(&ch) => {
                    self.find = Some((m, ch));
                    find(t, p, m, ch, n, false)
                }
            },
            ';' | ',' => match self.find {
                Some((f, ch)) => {
                    let f = if m == ',' {
                        match f {
                            'f' => 'F',
                            'F' => 'f',
                            't' => 'T',
                            _ => 't',
                        }
                    } else {
                        f
                    };
                    find(t, p, f, ch, n, true)
                }
                None => Mo::Bad,
            },
            '%' => {
                let Some(start) = (p..l1).find(|&i| "()[]{}".contains(t[i])) else {
                    return Mo::Bad;
                };
                match matching(t, start) {
                    Some(q) => Mo::To(q, Kind::Incl),
                    None => Mo::Bad,
                }
            }
            _ => Mo::Bad,
        }
    }
}

/// The keys of this frame as chars, removed from the input so the `TextEdit` doesn't see them.
fn take_keys(ctx: &egui::Context) -> Vec<char> {
    ctx.input_mut(|i| {
        let mut out = vec![];
        i.events.retain(|e| match e {
            Event::Text(t) => {
                out.extend(t.chars());
                false
            }
            Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } => {
                if !pressed {
                    return false;
                }
                let k = if modifiers.command || modifiers.ctrl {
                    match key {
                        Key::R => Some(CTRL_R),
                        Key::OpenBracket => Some(ESC),
                        Key::Z if modifiers.shift => Some(CTRL_R),
                        Key::Z => Some('u'),
                        _ => None,
                    }
                } else {
                    match key {
                        Key::Escape => Some(ESC),
                        Key::Enter => Some(ENTER),
                        Key::Backspace => Some(BACK),
                        Key::ArrowLeft => Some('h'),
                        Key::ArrowRight => Some('l'),
                        Key::ArrowUp => Some('k'),
                        Key::ArrowDown => Some('j'),
                        Key::Home => Some('0'),
                        Key::End => Some('$'),
                        Key::Delete => Some('x'),
                        _ => None,
                    }
                };
                out.extend(k);
                false
            }
            Event::Paste(_) | Event::Cut | Event::Ime(_) => false,
            _ => true,
        });
        out
    })
}

fn set_range(ctx: &egui::Context, id: Id, primary: usize, secondary: usize) {
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let (mut a, mut b) = (CCursor::default(), CCursor::default());
    a.index.0 = primary;
    b.index.0 = secondary;
    state.cursor.set_char_range(Some(CCursorRange {
        primary: a,
        secondary: b,
        h_pos: None,
    }));
    state.store(ctx, id);
}

/// A leading count, like the `3` of `3dw`.
fn count(keys: &[char], i: &mut usize) -> Option<usize> {
    if !keys.get(*i).is_some_and(|c| ('1'..='9').contains(c)) {
        return None;
    }
    let mut n = 0usize;
    while let Some(d) = keys.get(*i).and_then(|c| c.to_digit(10)) {
        n = n.saturating_mul(10).saturating_add(d as usize).min(100_000);
        *i += 1;
    }
    Some(n)
}

/// Start of the line of `p`.
fn ls(t: &[char], p: usize) -> usize {
    let p = p.min(t.len());
    t[..p].iter().rposition(|&c| c == '\n').map_or(0, |i| i + 1)
}

/// End of the line of `p` (its line break, or the end of the text).
fn le(t: &[char], p: usize) -> usize {
    let p = p.min(t.len());
    t[p..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(t.len(), |i| p + i)
}

fn line_no(t: &[char], p: usize) -> usize {
    t[..p.min(t.len())].iter().filter(|&&c| c == '\n').count()
}

fn nlines(t: &[char]) -> usize {
    line_no(t, t.len()) + 1
}

/// Start of line `n` (0-based), or of the last line.
fn line_start_n(t: &[char], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    t.iter()
        .enumerate()
        .filter(|(_, c)| **c == '\n')
        .nth(n - 1)
        .map_or_else(|| ls(t, t.len()), |(i, _)| i + 1)
}

/// First non-blank of the line starting at `s`.
fn fnb(t: &[char], s: usize) -> usize {
    let e = le(t, s);
    (s..e).find(|&i| !matches!(t[i], ' ' | '\t')).unwrap_or(e)
}

fn indent_of(t: &[char], s: usize) -> Vec<char> {
    let s = ls(t, s);
    t[s..fnb(t, s)].to_vec()
}

/// Normal mode keeps the cursor on a character, not after the end of the line.
fn clamp_normal(t: &[char], p: usize) -> usize {
    let p = p.min(t.len());
    if p > ls(t, p) && (p == t.len() || t[p] == '\n') {
        p - 1
    } else {
        p
    }
}

/// 0: blank, 1: word, 2: punctuation (big words: everything not blank is 1).
fn cls(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big || c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}

fn word_fwd(t: &[char], mut p: usize, big: bool) -> usize {
    let len = t.len();
    if p >= len {
        return len;
    }
    let c = cls(t[p], big);
    if c != 0 {
        while p < len && cls(t[p], big) == c {
            p += 1;
        }
    }
    while p < len && cls(t[p], big) == 0 {
        if t[p] == '\n' {
            p += 1;
            // an empty line is a word too
            if p < len && t[p] == '\n' {
                return p;
            }
            continue;
        }
        p += 1;
    }
    p
}

fn word_end(t: &[char], mut p: usize, big: bool) -> usize {
    let len = t.len();
    if len == 0 {
        return 0;
    }
    p += 1;
    while p < len && cls(t[p], big) == 0 {
        p += 1;
    }
    if p >= len {
        return len - 1;
    }
    let c = cls(t[p], big);
    while p + 1 < len && cls(t[p + 1], big) == c {
        p += 1;
    }
    p
}

fn word_back(t: &[char], mut p: usize, big: bool) -> usize {
    if p == 0 {
        return 0;
    }
    p = p.min(t.len()) - 1;
    while p > 0 && cls(t[p], big) == 0 {
        if t[p] == '\n' && t[p - 1] == '\n' {
            return p;
        }
        p -= 1;
    }
    let c = cls(t[p], big);
    while p > 0 && cls(t[p - 1], big) == c {
        p -= 1;
    }
    p
}

fn word_end_back(t: &[char], p: usize, big: bool) -> usize {
    let p = p.min(t.len());
    if p == 0 {
        return 0;
    }
    let c = t.get(p).map_or(0, |&ch| cls(ch, big));
    let mut q = p;
    // leave the current word
    while q > 0 && c != 0 && t.get(q).is_some_and(|&ch| cls(ch, big) == c) {
        q -= 1;
    }
    while q > 0 && cls(t[q], big) == 0 {
        q -= 1;
    }
    q
}

/// The next (or previous) empty line.
fn paragraph(t: &[char], p: usize, fwd: bool) -> usize {
    let empty = |i: usize| t[i] == '\n' && (i == 0 || t[i - 1] == '\n');
    let mut seen = false;
    let mut hit = |i: usize| {
        let stop = empty(i) && seen;
        seen |= t[i] != '\n';
        stop
    };
    if fwd {
        (p + 1..t.len()).find(|&i| hit(i)).unwrap_or(t.len())
    } else {
        (0..p.min(t.len())).rev().find(|&i| hit(i)).unwrap_or(0)
    }
}

fn find(t: &[char], p: usize, f: char, ch: char, n: usize, again: bool) -> Mo {
    let (l0, l1) = (ls(t, p), le(t, p));
    // `;` after `t` must not stay in front of the same character
    let skip = usize::from(again && matches!(f, 't' | 'T'));
    match f {
        'f' | 't' => {
            let from = p + 1 + skip;
            let hit = (from.min(l1)..l1).filter(|&i| t[i] == ch).nth(n - 1);
            match hit {
                Some(q) if f == 'f' => Mo::To(q, Kind::Incl),
                Some(q) => Mo::To(q - 1, Kind::Incl),
                None => Mo::Bad,
            }
        }
        _ => {
            let to = p.saturating_sub(skip).max(l0);
            let hit = (l0..to).rev().filter(|&i| t[i] == ch).nth(n - 1);
            match hit {
                Some(q) if f == 'F' => Mo::To(q, Kind::Excl),
                Some(q) => Mo::To(q + 1, Kind::Excl),
                None => Mo::Bad,
            }
        }
    }
}

/// The bracket matching the one at `p`.
fn matching(t: &[char], p: usize) -> Option<usize> {
    let pairs = [('(', ')'), ('[', ']'), ('{', '}')];
    let c = t[p];
    if let Some(&(o, cl)) = pairs.iter().find(|(o, _)| *o == c) {
        let mut depth = 0;
        for (i, &x) in t.iter().enumerate().skip(p) {
            if x == o {
                depth += 1;
            } else if x == cl {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    } else if let Some(&(o, cl)) = pairs.iter().find(|(_, cl)| *cl == c) {
        let mut depth = 0;
        for i in (0..=p).rev() {
            if t[i] == cl {
                depth += 1;
            } else if t[i] == o {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// The word characters around `p`.
fn word_at(t: &[char], p: usize) -> Option<(usize, usize)> {
    if p >= t.len() || cls(t[p], false) != 1 {
        return None;
    }
    let mut s = p;
    while s > 0 && cls(t[s - 1], false) == 1 {
        s -= 1;
    }
    let mut e = p;
    while e < t.len() && cls(t[e], false) == 1 {
        e += 1;
    }
    Some((s, e))
}

/// The `n`-th whole-word match of `word` after (or before) `p`, wrapping around.
fn search(t: &[char], p: usize, word: &[char], fwd: bool, n: usize) -> Option<usize> {
    let is_word = |i: usize| cls(t[i], false) == 1;
    let hits: Vec<usize> = (0..=t.len().saturating_sub(word.len()))
        .filter(|&i| {
            t[i..].starts_with(word)
                && (i == 0 || !is_word(i - 1))
                && (i + word.len() >= t.len() || !is_word(i + word.len()))
        })
        .collect();
    if hits.is_empty() {
        return None;
    }
    let mut q = p;
    for _ in 0..n {
        q = if fwd {
            *hits.iter().find(|&&h| h > q).unwrap_or(&hits[0])
        } else {
            *hits
                .iter()
                .rev()
                .find(|&&h| h < q)
                .unwrap_or(hits.last().unwrap())
        };
    }
    Some(q)
}

/// `iw`, `aw`, `i(`, `a"` …: the chars `s..e`.
fn text_object(t: &[char], p: usize, around: bool, o: char) -> Option<(usize, usize)> {
    let len = t.len();
    if p >= len && !matches!(o, 'w' | 'W') {
        return None;
    }
    match o {
        'w' | 'W' => {
            if len == 0 {
                return None;
            }
            let p = p.min(len - 1);
            let big = o == 'W';
            let c = cls(t[p], big);
            let same = |i: usize| t[i] != '\n' && cls(t[i], big) == c;
            let mut s = p;
            while s > 0 && same(s - 1) {
                s -= 1;
            }
            let mut e = p;
            while e < len && same(e) {
                e += 1;
            }
            if around && c != 0 {
                let mut e2 = e;
                while e2 < len && matches!(t[e2], ' ' | '\t') {
                    e2 += 1;
                }
                if e2 > e {
                    e = e2;
                } else {
                    while s > 0 && matches!(t[s - 1], ' ' | '\t') {
                        s -= 1;
                    }
                }
            }
            Some((s, e))
        }
        '(' | ')' | 'b' | '[' | ']' | '{' | '}' | 'B' | '<' | '>' => {
            let (op, cl) = match o {
                '(' | ')' | 'b' => ('(', ')'),
                '[' | ']' => ('[', ']'),
                '<' | '>' => ('<', '>'),
                _ => ('{', '}'),
            };
            let mut depth = 0;
            let mut open = None;
            for i in (0..=p).rev() {
                if t[i] == cl && i != p {
                    depth += 1;
                } else if t[i] == op {
                    if depth == 0 {
                        open = Some(i);
                        break;
                    }
                    depth -= 1;
                }
            }
            let open = open?;
            let mut depth = 0;
            let close = (open + 1..len).find(|&i| {
                if t[i] == op {
                    depth += 1;
                } else if t[i] == cl {
                    if depth == 0 {
                        return true;
                    }
                    depth -= 1;
                }
                false
            })?;
            if around {
                Some((open, close + 1))
            } else {
                Some((open + 1, close))
            }
        }
        '"' | '\'' | '`' => {
            let (l0, l1) = (ls(t, p), le(t, p));
            let quotes: Vec<usize> = (l0..l1).filter(|&i| t[i] == o).collect();
            let pairs = quotes.as_chunks::<2>().0;
            let pair = pairs
                .iter()
                .find(|q| q[0] <= p && p <= q[1])
                .or_else(|| pairs.iter().find(|q| q[0] > p))?;
            if around {
                Some((pair[0], pair[1] + 1))
            } else {
                Some((pair[0] + 1, pair[1]))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn run_keys(text: &str, p: usize, keys: &str) -> (String, usize, Mode) {
    let mut v = Vim::default();
    let mut ed = Ed::new(text, p);
    for k in keys.chars() {
        let k = if k == '⎋' { ESC } else { k };
        if v.mode == Mode::Insert {
            if k == ESC {
                v.leave_insert(&mut ed);
            } else {
                ed.splice(ed.p, ed.p, &[k]);
                ed.p += 1;
            }
        } else {
            v.press(&mut ed, k);
        }
    }
    let p = if v.mode == Mode::Normal {
        clamp_normal(&ed.t, ed.p)
    } else {
        ed.p
    };
    (ed.t.iter().collect(), p, v.mode)
}
