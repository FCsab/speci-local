use std::fmt::Write;

/// Which editor pane a diagnostic points into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Spec,
    Data,
    None,
}

/// A user-facing error: where, what, why, and how to fix it.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub pane: Pane,
    pub line: usize, // 1-based, 0 = no location
    pub col: usize,  // 1-based, in characters
    pub title: String,
    pub explanation: String,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(line: usize, col: usize, title: impl Into<String>) -> Self {
        Diagnostic {
            pane: Pane::Spec,
            line,
            col,
            title: title.into(),
            explanation: String::new(),
            hint: None,
        }
    }
    pub fn plain(title: impl Into<String>) -> Self {
        Diagnostic {
            pane: Pane::None,
            ..Self::new(0, 0, title)
        }
    }
    pub fn explain(mut self, s: impl Into<String>) -> Self {
        self.explanation = s.into();
        self
    }
    pub fn hint(mut self, s: impl Into<String>) -> Self {
        self.hint = Some(s.into());
        self
    }
    pub fn data(mut self) -> Self {
        self.pane = Pane::Data;
        self
    }

    /// Multi-line rendering with a source snippet and a caret under the column.
    pub fn render(&self, spec_src: &str, data_src: &str) -> String {
        let mut out = String::new();
        let where_ = match self.pane {
            Pane::Spec => "Specification",
            Pane::Data => "Test data",
            Pane::None => "",
        };
        if self.line > 0 {
            let _ = writeln!(
                out,
                "{where_}, line {}, column {}: {}",
                self.line, self.col, self.title
            );
            let src = if self.pane == Pane::Data {
                data_src
            } else {
                spec_src
            };
            if let Some(line) = src.lines().nth(self.line - 1) {
                let (text, col) = excerpt(line, self.col);
                let num = self.line.to_string();
                let _ = writeln!(out, " {num} | {text}");
                let _ = writeln!(out, " {} | {}^", " ".repeat(num.len()), " ".repeat(col - 1));
            }
        } else {
            let _ = writeln!(out, "{}", self.title);
        }
        if !self.explanation.is_empty() {
            let _ = writeln!(out, "{}", self.explanation);
        }
        if let Some(h) = &self.hint {
            let _ = writeln!(out, "Fix: {h}");
        }
        out.trim_end().to_string()
    }
}

const EXCERPT: usize = 160;

pub fn excerpt(line: &str, col: usize) -> (String, usize) {
    let len = line.chars().count();
    let col = col.clamp(1, len + 1);
    if len <= EXCERPT {
        return (line.to_string(), col);
    }
    let start = (col - 1).saturating_sub(EXCERPT / 2).min(len - EXCERPT);
    let mut text = String::new();
    if start > 0 {
        text.push('…');
    }
    text.extend(line.chars().skip(start).take(EXCERPT));
    if start + EXCERPT < len {
        text.push('…');
    }
    (text, col - start + usize::from(start > 0))
}

/// Levenshtein distance, for "did you mean" suggestions.
pub fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push(
                (prev[j] + (ca != *cb) as usize)
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1),
            );
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Closest candidate within a small edit distance.
pub fn suggest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (word.chars().count() / 3).max(1);
    let word = word.to_lowercase();
    candidates
        .into_iter()
        .map(|c| (distance(&word, &c.to_lowercase()), c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

pub fn did_you_mean<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> String {
    suggest(word, candidates)
        .map(|s| format!(" Did you mean `{s}`?"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_lines_are_cut_around_the_column() {
        assert_eq!(excerpt("n: 3", 4), ("n: 3".to_string(), 4));
        assert_eq!(excerpt("n: 3", 99).1, 5);
        assert_eq!(excerpt("", 0), (String::new(), 1));
        let line: String = (0..1000u32)
            .map(|i| char::from_u32(0x4E00 + i).unwrap())
            .collect();
        for col in [1, 2, 80, 81, 500, 920, 921, 1000, 1001] {
            let (text, at) = excerpt(&line, col);
            assert!(text.chars().count() <= EXCERPT + 2);
            let under = text.chars().nth(at - 1);
            assert_eq!(under, line.chars().nth(col - 1).or(None), "column {col}");
            assert_eq!(text.starts_with('…'), col > 81, "column {col}");
            assert_eq!(text.ends_with('…'), col < 921, "column {col}");
        }
        let data = format!("x: [{}]", vec!["1"; 5000].join(", "));
        let shown = Diagnostic::new(1, 9000, "bad").data().render("", &data);
        let lines: Vec<&str> = shown.lines().collect();
        assert!(lines[1].chars().count() < 200 && lines[2].ends_with('^'));
        assert_eq!(
            lines[1].chars().count(),
            lines[1].trim_end().chars().count()
        );
        let caret = lines[2].chars().count() - 1;
        assert_eq!(lines[1].chars().nth(caret), data.chars().nth(8999));
    }

    #[test]
    fn edit_distance() {
        for (a, b, d) in [
            ("", "", 0),
            ("abc", "abc", 0),
            ("", "abc", 3),
            ("kitten", "sitting", 3),
            ("szamok", "számok", 1),
            ("ab", "ba", 2),
        ] {
            assert_eq!(distance(a, b), d, "{a} → {b}");
            assert_eq!(distance(b, a), d, "{b} → {a}");
        }
    }

    #[test]
    fn suggestions_are_close_and_case_blind() {
        let names = ["szamok", "db", "n"];
        assert_eq!(suggest("szamko", names), Some("szamok"));
        assert_eq!(suggest("SZAMOK", names), Some("szamok"));
        assert_eq!(suggest("dc", names), Some("db"));
        assert_eq!(suggest("xyz", names), None);
        assert_eq!(suggest("anything", []), None);
        assert_eq!(suggest("Post", ["Pre", "Post", "Pos"]), Some("Post"));
        assert_eq!(did_you_mean("szamko", names), " Did you mean `szamok`?");
        assert_eq!(did_you_mean("xyz", names), "");
    }

    #[test]
    fn builders_fill_in_the_parts() {
        let d = Diagnostic::new(3, 5, "Title").explain("Why").hint("How");
        assert_eq!((d.pane, d.line, d.col), (Pane::Spec, 3, 5));
        assert_eq!(
            d.render("a\nb\ncdefgh", ""),
            "Specification, line 3, column 5: Title\n 3 | cdefgh\n   |     ^\nWhy\nFix: How"
        );
        assert_eq!(
            d.render("a", ""),
            "Specification, line 3, column 5: Title\nWhy\nFix: How"
        );
        let d = Diagnostic::new(1, 1, "T").data();
        assert_eq!(d.pane, Pane::Data);
        assert_eq!(
            d.render("spec", "data"),
            "Test data, line 1, column 1: T\n 1 | data\n   | ^"
        );
        let d = Diagnostic::plain("No position");
        assert_eq!((d.pane, d.line), (Pane::None, 0));
        assert_eq!(d.render("spec", "data"), "No position");
    }
}
