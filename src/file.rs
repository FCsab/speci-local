//! `.sp` files: a whole project in one file. The specification is plain text; the test cases
//! and the structograms (the web Stuki's JSON, see [`crate::stuki::format`]) are embedded as
//! block comments, so the whole file is still a valid specification and can be pasted into the
//! ELTE editor as-is.
//!
//! ```text
//! // title: Odd numbers
//! Be:
//! n∈N
//! ...
//! /*--- test ---
//! n: 5
//! ---*/
//!
//! /*--- stuki ---
//! { "nodes": { ... }, "structograms": [ ... ], ... }
//! ---*/
//! ```

use crate::compact;
use crate::stuki::{format as stuki_format, model::Program};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct Doc {
    pub title: String,
    pub spec: String,
    pub tests: Vec<String>,
}

/// A whole project: the specification with its test cases, and the structograms.
#[derive(Clone, Debug, Default)]
pub struct Project {
    pub doc: Doc,
    /// `None`: the project has no structograms of its own (the file has no Stuki block).
    pub stuki: Option<Program>,
}

const TITLE: &str = "// title: ";
const OPEN: &str = "/*--- test ---\n";
const STUKI_OPEN: &str = "/*--- stuki ---\n";
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

/// The whole project as a `.sp` file. Untouched structograms (the empty "Main" every project
/// starts with) are left out, so a project without structograms is a plain `.sp` file.
pub fn to_project(doc: &Doc, stuki: &Program) -> String {
    let mut out = to_sp(doc);
    if !stuki.content_is(&Program::default().content()) {
        // `*` only occurs inside JSON strings, where `\/` is the same as `/`
        let json = stuki_format::to_st(stuki).replace("*/", "*\\/");
        out.push_str(&format!("\n{STUKI_OPEN}{}\n{CLOSE}\n", json.trim_end()));
    }
    out
}

/// Read a `.sp` file. A damaged Stuki block is an error; the specification and test cases are
/// still in [`from_sp`].
pub fn from_project(text: &str) -> Result<Project, String> {
    let (doc, stuki) = split(text);
    let stuki = match stuki {
        Some(json) => Some(
            stuki_format::from_st(&json)
                .map_err(|e| format!("the structograms in it can't be read ({e})"))?,
        ),
        None => None,
    };
    Ok(Project { doc, stuki })
}

/// The specification and test cases of a `.sp` file (its structograms left out).
pub fn from_sp(text: &str) -> Doc {
    split(text).0
}

/// The specification and test cases, and the Stuki block's JSON.
fn split(text: &str) -> (Doc, Option<String>) {
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
    let mut stuki = None;
    // the next test or Stuki block, whichever comes first
    let next = |rest: &str| {
        [OPEN, STUKI_OPEN]
            .into_iter()
            .filter_map(|open| Some((rest.find(open)?, open)))
            .min()
    };
    while let Some((i, open)) = next(rest) {
        spec.push_str(&rest[..i]);
        let after = &rest[i + open.len()..];
        let end = after.find(CLOSE).unwrap_or(after.len());
        let body = after[..end].trim_end();
        if open == OPEN {
            tests.push(format!("{body}\n"));
        } else {
            stuki = Some(body.to_string());
        }
        rest = after.get(end + CLOSE.len()..).unwrap_or("");
    }
    spec.push_str(rest);
    if tests.is_empty() {
        tests.push(String::new());
    }
    let doc = Doc {
        title,
        spec: format!("{}\n", spec.trim_end()),
        tests,
    };
    (doc, stuki)
}

// ---------------------------------------------------------------- clipboard

/// Prefix of the compact clipboard form. The version lets the format change later.
pub const COMPACT_PREFIX: &str = "SPECI1:";

/// The whole project as one short line: `SPECI1:` + base64url(deflate(.sp text)).
pub fn to_compact(doc: &Doc, stuki: &Program) -> String {
    compact::pack(COMPACT_PREFIX, &to_project(doc, stuki))
}

/// A link that opens the project in the ELTE specification editor. The web app keeps no title,
/// so it goes in as the first comment line, as in a `.sp` file.
pub fn to_web_link(doc: &Doc) -> String {
    let mut spec = String::new();
    if !doc.title.trim().is_empty() {
        spec = format!("{TITLE}{}\n", doc.title.trim());
    }
    spec.push_str(&doc.spec);
    // result 0: pending, the web app runs the tests itself
    let tests: Vec<_> = doc
        .tests
        .iter()
        .map(|t| serde_json::json!({"data": t, "name": "", "result": 0}))
        .collect();
    let json = serde_json::json!({"specification": spec, "tests": tests});
    compact::web_link(compact::SPEC_EDITOR, &json.to_string())
}

/// Read a project back from clipboard text: the compact form, or plain `.sp` / specification text.
pub fn from_clipboard(text: &str) -> Result<Project, String> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix(COMPACT_PREFIX) {
        return from_project(&compact::unpack(COMPACT_PREFIX, rest, "project")?)
            .map_err(|e| format!("The copied project is damaged: {e}."));
    }
    // a specification may well say `data=`, so only links to the web editor count
    if t.contains("/specifikacio") {
        if let Some(json) = compact::web_link_json(t) {
            return from_web_json(&json?).map(|doc| Project { doc, stuki: None });
        }
        if t.contains("uuid=") {
            return Err("This link points to a specification saved on the university server, which can't be read without logging in. Open it in the web editor and share it from there (a link with data=).".into());
        }
    }
    let looks_like_spec = t.lines().any(|l| {
        let l = l.trim_start();
        ["Be:", "In:", "Ki:", "Out:", "Uf:", "Post:"]
            .iter()
            .any(|h| l.starts_with(h))
    });
    if looks_like_spec {
        from_project(t).map_err(|e| format!("The pasted project is damaged: {e}."))
    } else if t.is_empty() {
        Err("The clipboard is empty.".into())
    } else {
        Err("The clipboard doesn't contain a project: expected text starting with SPECI1: or a specification with blocks like Be:/Uf:.".into())
    }
}

/// A project as the web editor keeps it: `{"specification": …, "tests": [{"data": …}, …]}`.
/// A title exported from here comes back from its comment line.
fn from_web_json(json: &str) -> Result<Doc, String> {
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("The link's data is not a specification: {e}"))?;
    let spec = v["specification"]
        .as_str()
        .ok_or("The link's data has no specification.")?;
    let mut doc = from_sp(spec);
    let tests: Vec<String> = v["tests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["data"].as_str())
        .map(|t| t.replace("\r\n", "\n"))
        .collect();
    if !tests.is_empty() {
        doc.tests = tests;
    }
    Ok(doc)
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
        assert_eq!(
            from_clipboard(plain).unwrap().doc.spec,
            format!("{plain}\n")
        );
        let e = from_clipboard("just some text").unwrap_err();
        assert!(e.contains("doesn't contain a project"), "{e}");
        assert!(from_clipboard("  \n ").unwrap_err().contains("empty"));
    }

    #[test]
    fn web_link_holds_what_the_spec_editor_reads() {
        let doc = Doc {
            title: "Párosak".into(),
            spec: "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n".into(),
            tests: vec!["n: 1\nx: 1\n".into(), "n: 2\n".into()],
        };
        let link = to_web_link(&doc);
        let data = link
            .strip_prefix(compact::SPEC_EDITOR)
            .and_then(|l| l.strip_prefix("?data="))
            .unwrap()
            .replace("%2B", "+")
            .replace("%2F", "/")
            .replace("%3D", "=");
        let gz = compact::b64_decode(&data).unwrap();
        let json = compact::inflate(&gz[10..gz.len() - 8]).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(
            v["specification"],
            "// title: Párosak\nBe: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n"
        );
        assert_eq!(v["tests"][1]["data"], "n: 2\n");
        assert_eq!(v["tests"][0]["name"], "");
        assert_eq!(v["tests"][0]["result"], 0);
        assert_eq!(from_clipboard(&link).unwrap().doc, doc);
    }

    #[test]
    fn web_editor_links_paste() {
        // as the web editor saves on Windows, without a title
        let json = r#"{"specification":"Be: n∈N\r\nUf: data=n","tests":[{"data":"n: 1\r\n","name":"","result":1}]}"#;
        let link = compact::web_link(compact::SPEC_EDITOR, json);
        let doc = from_clipboard(&link).unwrap().doc;
        assert_eq!(doc.title, "");
        assert_eq!(doc.spec, "Be: n∈N\nUf: data=n\n");
        assert_eq!(doc.tests, ["n: 1\n"]);
        // a specification that says data= is still a specification
        assert_eq!(from_clipboard(&doc.spec).unwrap().doc.spec, doc.spec);
        let uuid_link = format!("{}?uuid=1234", compact::SPEC_EDITOR);
        assert!(
            from_clipboard(&uuid_link)
                .unwrap_err()
                .contains("logging in")
        );
        let damaged = format!("{}?data=@@", compact::SPEC_EDITOR);
        assert!(from_clipboard(&damaged).unwrap_err().contains("damaged"));
    }

    #[test]
    fn compact_round_trip_and_errors() {
        let doc = Doc {
            title: "Karácsonyfák".into(),
            spec: "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n".into(),
            tests: vec!["n: 1\nx: 1\n".into(), "n: 2\n".into()],
        };
        let c = to_compact(&doc, &Program::default());
        assert!(c.starts_with(COMPACT_PREFIX) && !c.contains(char::is_whitespace));
        assert_eq!(from_clipboard(&c).unwrap().doc, doc);
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
        assert_eq!(
            from_clipboard(&format!("\n  {wrapped}  \n")).unwrap().doc,
            doc
        );
        // plain .sp text works too
        assert_eq!(from_clipboard(&to_sp(&doc)).unwrap().doc, doc);
        // damage is reported, not panicked on
        assert!(from_clipboard(&c[..c.len() - 6]).is_err());
        assert!(from_clipboard("SPECI1:@@@").is_err());
        assert!(from_clipboard("hello").is_err());
        assert!(from_clipboard("").is_err());
    }

    #[test]
    fn projects_keep_their_structograms() {
        use crate::stuki::rich::Rich;
        let doc = Doc {
            title: "Összeg".into(),
            spec: "Be: n∈N\nKi: s∈N\nEf: -\nUf: s=n\n".into(),
            tests: vec!["n: 1\ns: 1\n".into(), "n: 2\n".into()],
        };
        // the empty structogram every project starts with is not written
        let plain = to_project(&doc, &Program::default());
        assert_eq!(plain, to_sp(&doc));
        assert!(from_project(&plain).unwrap().stuki.is_none());

        let mut stuki = Program::default();
        stuki.diagrams[0].name = Rich::plain("Összeg");
        stuki.diagrams[0].variables = Rich::plain("a*/b"); // can't close the comment
        let text = to_project(&doc, &stuki);
        assert_eq!(text.matches("*/").count(), 3, "{text}");
        assert!(text.ends_with("\n---*/\n"));
        let back = from_project(&text).unwrap();
        assert_eq!(back.doc, doc);
        assert_eq!(back.stuki.unwrap().content(), stuki.content());
        assert_eq!(from_sp(&text), doc);
        // still a valid specification: everything else is in comments
        assert!(crate::parse(&text).is_ok());
        // Windows line endings
        let back = from_project(&text.replace('\n', "\r\n")).unwrap();
        assert_eq!(back.stuki.unwrap().content(), stuki.content());
        // the clipboard line carries the structograms too
        let pasted = from_clipboard(&to_compact(&doc, &stuki)).unwrap();
        assert_eq!(pasted.doc, doc);
        assert_eq!(pasted.stuki.unwrap().content(), stuki.content());

        let damaged = text.replace("\"structograms\"", "\"x\"");
        let e = from_project(&damaged).unwrap_err();
        assert!(e.contains("structograms"), "{e}");
        assert_eq!(from_sp(&damaged), doc);
    }
}
