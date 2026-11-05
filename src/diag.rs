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
            if let Some(text) = src.lines().nth(self.line - 1) {
                let num = self.line.to_string();
                let _ = writeln!(out, " {num} | {text}");
                let _ = writeln!(
                    out,
                    " {} | {}^",
                    " ".repeat(num.len()),
                    " ".repeat(self.col.saturating_sub(1))
                );
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
    candidates
        .into_iter()
        .map(|c| (distance(&word.to_lowercase(), &c.to_lowercase()), c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

pub fn did_you_mean<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> String {
    suggest(word, candidates)
        .map(|s| format!(" Did you mean `{s}`?"))
        .unwrap_or_default()
}
