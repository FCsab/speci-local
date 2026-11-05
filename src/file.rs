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
    let packed = miniz_oxide::deflate::compress_to_vec(to_sp(doc).as_bytes(), 9);
    format!("{COMPACT_PREFIX}{}", b64_encode(&packed))
}

/// Read a project back from clipboard text: the compact form, or plain `.sp` / specification text.
pub fn from_clipboard(text: &str) -> Result<Doc, String> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix(COMPACT_PREFIX) {
        // chat apps and mail clients like to wrap long lines
        let rest: String = rest.chars().filter(|c| !c.is_whitespace()).collect();
        let packed = b64_decode(&rest).ok_or(
            "The text after SPECI1: is damaged (not valid base64). Was it copied completely?",
        )?;
        let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, 16 << 20).map_err(
            |_| {
                "The copied project is damaged or incomplete. Copy it again from the source."
                    .to_string()
            },
        )?;
        let sp = String::from_utf8(raw)
            .map_err(|_| "The copied project is not valid text.".to_string())?;
        return Ok(from_sp(&sp));
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

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding (URL- and chat-safe).
fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | (*b as u32) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        // also accept the standard alphabet (+ /)
        let v = match c {
            b'+' => 62,
            b'/' => 63,
            _ => B64.iter().position(|&x| x == c)? as u32,
        };
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        for len in 0..40 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(b64_decode(&b64_encode(&data)).unwrap(), data, "len {len}");
        }
        assert_eq!(b64_encode(b"Man"), "TWFu");
        assert_eq!(b64_encode(b"Ma"), "TWE");
        assert!(b64_decode("TW!u").is_none());
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
