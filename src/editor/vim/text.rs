//! Positions in the text: lines, words, brackets, searches and text objects.

use super::*;

/// A leading count, like the `3` of `3dw`.
pub(super) fn count(keys: &[char], i: &mut usize) -> Option<usize> {
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
pub(super) fn ls(t: &[char], p: usize) -> usize {
    let p = p.min(t.len());
    t[..p].iter().rposition(|&c| c == '\n').map_or(0, |i| i + 1)
}

/// End of the line of `p` (its line break, or the end of the text).
pub(super) fn le(t: &[char], p: usize) -> usize {
    let p = p.min(t.len());
    t[p..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(t.len(), |i| p + i)
}

pub(super) fn line_no(t: &[char], p: usize) -> usize {
    t[..p.min(t.len())].iter().filter(|&&c| c == '\n').count()
}

pub(super) fn nlines(t: &[char]) -> usize {
    line_no(t, t.len()) + 1
}

/// Start of line `n` (0-based), or of the last line.
pub(super) fn line_start_n(t: &[char], n: usize) -> usize {
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
pub(super) fn fnb(t: &[char], s: usize) -> usize {
    let e = le(t, s);
    (s..e).find(|&i| !matches!(t[i], ' ' | '\t')).unwrap_or(e)
}

pub(super) fn indent_of(t: &[char], s: usize) -> Vec<char> {
    let s = ls(t, s);
    t[s..fnb(t, s)].to_vec()
}

/// Normal mode keeps the cursor on a character, not after the end of the line.
pub(super) fn clamp_normal(t: &[char], p: usize) -> usize {
    let p = p.min(t.len());
    if p > ls(t, p) && (p == t.len() || t[p] == '\n') {
        p - 1
    } else {
        p
    }
}

/// 0: blank, 1: word, 2: punctuation (big words: everything not blank is 1).
pub(super) fn cls(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big || c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}

pub(super) fn word_fwd(t: &[char], mut p: usize, big: bool) -> usize {
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

pub(super) fn word_end(t: &[char], mut p: usize, big: bool) -> usize {
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

pub(super) fn word_back(t: &[char], mut p: usize, big: bool) -> usize {
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

pub(super) fn word_end_back(t: &[char], p: usize, big: bool) -> usize {
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
pub(super) fn paragraph(t: &[char], p: usize, fwd: bool) -> usize {
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

pub(super) fn find(t: &[char], p: usize, f: char, ch: char, n: usize, again: bool) -> Mo {
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
pub(super) fn matching(t: &[char], p: usize) -> Option<usize> {
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
pub(super) fn word_at(t: &[char], p: usize) -> Option<(usize, usize)> {
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
pub(super) fn search(t: &[char], p: usize, word: &[char], fwd: bool, n: usize) -> Option<usize> {
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
pub(super) fn text_object(t: &[char], p: usize, around: bool, o: char) -> Option<(usize, usize)> {
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
