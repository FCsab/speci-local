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

use crate::compact;
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
    // Windows editors may add a byte-order mark and CRLF line endings
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
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

// ---------------------------------------------------------------- clipboard

/// Prefix of the compact clipboard form. The version lets the format change later.
pub const COMPACT_PREFIX: &str = "SPECI1:";

/// The whole project as one short line: `SPECI1:` + base64url(deflate(.sp text)).
pub fn to_compact(doc: &Doc) -> String {
    compact::pack(COMPACT_PREFIX, &to_sp(doc))
}

/// Read a project back from clipboard text: the compact form, or plain `.sp` / specification text.
pub fn from_clipboard(text: &str) -> Result<Doc, String> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix(COMPACT_PREFIX) {
        return Ok(from_sp(&compact::unpack(COMPACT_PREFIX, rest, "project")?));
    }
    let looks_like_spec = t.lines().any(|l| {
        let l = l.trim_start();
        ["Be:", "In:", "Ki:", "Out:", "Uf:", "Post:"]
            .iter()
            .any(|h| l.starts_with(h))
    });
    if looks_like_spec {
        Ok(from_sp(t))
    } else if t.is_empty() {
        Err("The clipboard is empty.".into())
    } else {
        Err("The clipboard doesn't contain a project: expected text starting with SPECI1: or a specification with blocks like Be:/Uf:.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sp_files_keep_title_spec_and_tests() {
        let doc = Doc {
            title: "Páros számok".into(),
            spec: "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n".into(),
            tests: vec!["n: 1\nx: 1\n".into(), "n: 2\nx: 2\n".into()],
        };
        let sp = to_sp(&doc);
        assert!(sp.starts_with("// title: Páros számok\nBe: n∈N\n"));
        assert_eq!(sp.matches("/*--- test ---\n").count(), 2);
        assert_eq!(from_sp(&sp), doc);
        // the whole file is a valid specification: the tests are comments
        assert!(crate::parse(&sp).is_ok());
        let windows = format!("\u{feff}{}", sp.replace('\n', "\r\n"));
        assert_eq!(from_sp(&windows), doc);
    }

    #[test]
    fn sp_files_without_title_or_tests() {
        let doc = from_sp("Be: n∈N\n\n\n");
        assert_eq!(doc.title, "");
        assert_eq!(doc.spec, "Be: n∈N\n");
        assert_eq!(doc.tests, [""]);
        assert!(to_sp(&doc).starts_with("Be: n∈N\n"));
        let doc = from_sp("Be: n∈N\n/*--- test ---\nn: 1\n");
        assert_eq!(doc.tests, ["n: 1\n"]);
    }

    #[test]
    fn comment_end_inside_a_test_cannot_close_it() {
        let doc = Doc {
            title: String::new(),
            spec: "Be: s∈S\n".into(),
            tests: vec!["s: \"a*/b\"\n".into()],
        };
        let sp = to_sp(&doc);
        assert_eq!(sp.matches("*/").count(), 1, "{sp}");
        assert_eq!(from_sp(&sp).tests, ["s: \"a* /b\"\n"]);
    }

    #[test]
    fn clipboard_text_is_recognised() {
        let plain = "// my notes\n  Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n";
        assert_eq!(from_clipboard(plain).unwrap().spec, format!("{plain}\n"));
        let e = from_clipboard("just some text").unwrap_err();
        assert!(e.contains("doesn't contain a project"), "{e}");
        assert!(from_clipboard("  \n ").unwrap_err().contains("empty"));
    }

    #[test]
    fn compact_round_trip_and_errors() {
        let doc = Doc {
            title: "Karácsonyfák".into(),
            spec: "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n".into(),
            tests: vec!["n: 1\nx: 1\n".into(), "n: 2\n".into()],
        };
        let c = to_compact(&doc);
        assert!(c.starts_with(COMPACT_PREFIX) && !c.contains(char::is_whitespace));
        assert_eq!(from_clipboard(&c).unwrap(), doc);
        // wrapped by a chat app, surrounded by blank lines
        let wrapped: String = c
            .chars()
            .enumerate()
            .flat_map(|(i, ch)| {
                if i % 20 == 19 {
                    vec![ch, '\n']
                } else {
                    vec![ch]
                }
            })
            .collect();
        assert_eq!(from_clipboard(&format!("\n  {wrapped}  \n")).unwrap(), doc);
        // plain .sp text works too
        assert_eq!(from_clipboard(&to_sp(&doc)).unwrap(), doc);
        // damage is reported, not panicked on
        assert!(from_clipboard(&c[..c.len() - 6]).is_err());
        assert!(from_clipboard("SPECI1:@@@").is_err());
        assert!(from_clipboard("hello").is_err());
        assert!(from_clipboard("").is_err());
    }
}
