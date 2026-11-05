//! `.sp` files: the specification as plain text, test cases embedded as block comments.
//! The whole file is still a valid specification, so it can be pasted into the ELTE editor as-is.
//!
//! ```text
//! // title: Odd numbers
//! Be:
//! n∈N
//! ...
//! /*--- test ---
//! n: 5
//! ---*/
//! ```

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct Doc {
    pub title: String,
    pub spec: String,
    pub tests: Vec<String>,
}

const TITLE: &str = "// title: ";
const OPEN: &str = "/*--- test ---\n";
const CLOSE: &str = "---*/";

pub fn to_sp(doc: &Doc) -> String {
    let mut out = String::new();
    if !doc.title.trim().is_empty() {
        out.push_str(&format!("{TITLE}{}\n", doc.title.trim()));
    }
    out.push_str(doc.spec.trim_end());
    out.push('\n');
    for t in &doc.tests {
        // `*/` inside a test would end the comment early
        let body = t.trim_end().replace("*/", "* /");
        out.push_str(&format!("\n{OPEN}{body}\n{CLOSE}\n"));
    }
    out
}

pub fn from_sp(text: &str) -> Doc {
    let text = text.replace("\r\n", "\n");
    let mut rest = text.as_str();
    let mut title = String::new();
    if let Some(r) = rest.strip_prefix(TITLE) {
        let end = r.find('\n').unwrap_or(r.len());
        title = r[..end].trim().to_string();
        rest = r.get(end + 1..).unwrap_or("");
    }
    let mut spec = String::new();
    let mut tests = Vec::new();
    while let Some(i) = rest.find(OPEN) {
        spec.push_str(&rest[..i]);
        let after = &rest[i + OPEN.len()..];
        let end = after.find(CLOSE).unwrap_or(after.len());
        tests.push(format!("{}\n", after[..end].trim_end()));
        rest = after.get(end + CLOSE.len()..).unwrap_or("");
    }
    spec.push_str(rest);
    if tests.is_empty() {
        tests.push(String::new());
    }
    Doc {
        title,
        spec: format!("{}\n", spec.trim_end()),
        tests,
    }
}
