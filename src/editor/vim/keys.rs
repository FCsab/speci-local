//! The commands of Normal and Visual mode, and the motions.

use super::*;

impl Vim {
    /// Normal mode: returns false while the command needs more keys.
    pub(super) fn normal(&mut self, ed: &mut Ed, keys: &[char]) -> bool {
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
    pub(super) fn operator(
        &mut self,
        ed: &mut Ed,
        op: char,
        n1: Option<usize>,
        rest: &[char],
    ) -> bool {
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
    pub(super) fn visual(&mut self, ed: &mut Ed, keys: &[char]) -> bool {
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

    pub(super) fn motion(
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
