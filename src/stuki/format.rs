//! Structograms on disk and on the clipboard: the web Stuki's own state format (a flat `nodes`
//! map plus `structograms`), so `.st` files and the web app's `?data=` links can be moved between
//! the two, and the compact `STUKI1:` line.

use super::model::{Case, Diagram, Kind, Node, Palettes, Program, fresh_id};
use super::rich::Rich;
use crate::compact;
use serde_json::{Map, Value, json};

/// The web Stuki's state: `{ nodes: {id: node}, structograms: [...], colors: {...} }`.
pub fn to_json(p: &Program) -> Value {
    struct W {
        nodes: Map<String, Value>,
        n: usize,
    }
    impl W {
        fn id(&mut self) -> String {
            self.n += 1;
            format!("n{:07}", self.n)
        }
        fn seq(&mut self, seq: &[Node], parent: Option<&str>) -> String {
            let id = self.id();
            let children: Vec<Value> = seq.iter().map(|n| Value::from(self.node(n, &id))).collect();
            self.nodes.insert(
                id.clone(),
                json!({"id": id, "parentId": parent, "hidden": false, "type": "sequence", "childrenIds": children}),
            );
            id
        }
        fn node(&mut self, n: &Node, parent: &str) -> String {
            let id = self.id();
            let mut v = match &n.kind {
                Kind::Noop => json!({"type": "noop"}),
                Kind::Statement(t) => json!({"type": "statement", "text": t.to_html()}),
                Kind::If { cond, yes, no } => json!({
                    "type": "if",
                    "condition": cond.to_html(),
                    "trueBranchSequenceId": self.seq(yes, Some(&id)),
                    "falseBranchSequenceId": self.seq(no, Some(&id)),
                }),
                Kind::While { cond, body } => json!({
                    "type": "while", "condition": cond.to_html(), "bodySequenceId": self.seq(body, Some(&id)),
                }),
                Kind::Repeat { cond, body } => json!({
                    "type": "repeat", "condition": cond.to_html(), "bodySequenceId": self.seq(body, Some(&id)),
                }),
                Kind::Switch { cases } => {
                    let cases: Vec<Value> = cases
                        .iter()
                        .map(|c| {
                            let cid = self.id();
                            let body = self.seq(&c.body, Some(&id));
                            match &c.cond {
                                Some(t) => json!({"id": cid, "type": "case", "condition": t.to_html(), "bodySequenceId": body}),
                                None => json!({"id": cid, "type": "default", "bodySequenceId": body}),
                            }
                        })
                        .collect();
                    json!({"type": "switch", "cases": cases})
                }
            };
            let o = v.as_object_mut().unwrap();
            o.insert("id".into(), id.clone().into());
            o.insert("parentId".into(), parent.into());
            o.insert("hidden".into(), false.into());
            if let Some(bg) = &n.bg {
                o.insert("backgroundColor".into(), bg.clone().into());
            }
            self.nodes.insert(id.clone(), v);
            id
        }
    }
    let mut w = W {
        nodes: Map::new(),
        n: 0,
    };
    let structograms: Vec<Value> = p
        .diagrams
        .iter()
        .map(|d| {
            let root = w.seq(&d.body, None);
            let mut v = json!({
                "name": d.name.to_html(),
                "returnText": d.ret.to_html(),
                "variables": d.variables.to_html(),
                "rootId": root,
                "showHeader": d.show_header,
                "showVariables": d.show_variables,
                "showReturnText": d.show_return,
            });
            if let Some(bg) = &d.bg {
                v.as_object_mut()
                    .unwrap()
                    .insert("backgroundColor".into(), bg.clone().into());
            }
            v
        })
        .collect();
    let c = &p.colors;
    json!({
        "id": "0",
        "hoverId": null,
        "selectedId": null,
        "nodes": w.nodes,
        "structograms": structograms,
        "colors": {
            "text": c.text,
            "textBackground": c.text_background,
            "nodeBackground": c.node_background,
            "structogramBackground": c.structogram_background,
        },
    })
}

/// Read the web Stuki's state (from a `.st` file, a copied project or a `?data=` link).
pub fn from_json(v: &Value) -> Result<Program, String> {
    let nodes = v
        .get("nodes")
        .and_then(Value::as_object)
        .ok_or("not a structogram: the \"nodes\" list is missing")?;
    let structograms = v
        .get("structograms")
        .and_then(Value::as_array)
        .ok_or("not a structogram: the \"structograms\" list is missing")?;
    let text = |o: &Value, k: &str| Rich::from_html(o.get(k).and_then(Value::as_str).unwrap_or(""));
    let flag = |o: &Value, k: &str, d: bool| o.get(k).and_then(Value::as_bool).unwrap_or(d);
    let color = |o: &Value| {
        o.get("backgroundColor")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    const MAX_DEPTH: usize = 100;
    const MAX_ELEMENTS: usize = 100_000;

    fn seq(
        nodes: &Map<String, Value>,
        id: Option<&str>,
        depth: usize,
        left: &mut usize,
    ) -> Result<Vec<Node>, String> {
        if depth > MAX_DEPTH {
            return Err("the structogram is nested too deeply (or is damaged)".into());
        }
        let mut out = Vec::new();
        let s = id.and_then(|id| nodes.get(id));
        let children = s
            .and_then(|s| s.get("childrenIds"))
            .and_then(Value::as_array);
        for c in children.into_iter().flatten() {
            let Some(n) = c.as_str().and_then(|c| nodes.get(c)) else {
                continue;
            };
            if let Some(n) = node(nodes, n, depth + 1, left)? {
                out.push(n);
            }
        }
        if out.is_empty() {
            out.push(Node::noop());
        }
        Ok(out)
    }
    fn node(
        nodes: &Map<String, Value>,
        n: &Value,
        depth: usize,
        left: &mut usize,
    ) -> Result<Option<Node>, String> {
        *left = left
            .checked_sub(1)
            .ok_or("the structogram has too many elements (or is damaged)")?;
        let t = |k: &str| Rich::from_html(n.get(k).and_then(Value::as_str).unwrap_or(""));
        let mut sub =
            |of: &Value, k: &str| seq(nodes, of.get(k).and_then(Value::as_str), depth, left);
        let kind = match n.get("type").and_then(Value::as_str).unwrap_or("") {
            "noop" => Kind::Noop,
            "statement" => Kind::Statement(t("text")),
            "if" => Kind::If {
                cond: t("condition"),
                yes: sub(n, "trueBranchSequenceId")?,
                no: sub(n, "falseBranchSequenceId")?,
            },
            "while" => Kind::While {
                cond: t("condition"),
                body: sub(n, "bodySequenceId")?,
            },
            "repeat" => Kind::Repeat {
                cond: t("condition"),
                body: sub(n, "bodySequenceId")?,
            },
            "switch" => {
                let mut cases = Vec::new();
                for c in n
                    .get("cases")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let default = c.get("type").and_then(Value::as_str) == Some("default");
                    cases.push(Case {
                        id: fresh_id(),
                        cond: (!default).then(|| {
                            Rich::from_html(
                                c.get("condition").and_then(Value::as_str).unwrap_or(""),
                            )
                        }),
                        body: sub(c, "bodySequenceId")?,
                    });
                }
                // the default case belongs at the end
                cases.sort_by_key(|c| c.cond.is_none());
                if cases.is_empty() {
                    return Ok(None);
                }
                Kind::Switch { cases }
            }
            _ => return Ok(None), // drag & drop placeholders and anything unknown
        };
        let bg = n
            .get("backgroundColor")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        Ok(Some(Node {
            id: fresh_id(),
            bg,
            kind,
        }))
    }

    let mut left = MAX_ELEMENTS;
    let mut diagrams = Vec::new();
    for s in structograms {
        diagrams.push(Diagram {
            id: fresh_id(),
            name: text(s, "name"),
            variables: text(s, "variables"),
            ret: text(s, "returnText"),
            show_header: flag(s, "showHeader", false),
            show_variables: flag(s, "showVariables", false),
            show_return: flag(s, "showReturnText", true),
            bg: color(s),
            body: seq(nodes, s.get("rootId").and_then(Value::as_str), 0, &mut left)?,
        });
    }
    let mut colors = Palettes::default();
    if let Some(c) = v.get("colors") {
        let list = |k: &str, into: &mut Vec<String>| {
            if let Some(a) = c.get(k).and_then(Value::as_array) {
                *into = a
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect();
            }
        };
        list("text", &mut colors.text);
        list("textBackground", &mut colors.text_background);
        list("nodeBackground", &mut colors.node_background);
        list("structogramBackground", &mut colors.structogram_background);
    }
    Ok(Program { diagrams, colors })
}

// ---------------------------------------------------------------- files and the clipboard

/// `.st` file: the web app's state as pretty-printed JSON.
pub fn to_st(p: &Program) -> String {
    let mut s = serde_json::to_string_pretty(&to_json(p)).unwrap_or_default();
    s.push('\n');
    s
}

pub fn from_st(text: &str) -> Result<Program, String> {
    let text = text.trim_start_matches('\u{feff}');
    let v: Value =
        serde_json::from_str(text).map_err(|e| format!("not a structogram file: {e}"))?;
    from_json(&v)
}

/// Prefix of the compact clipboard form.
pub const COMPACT_PREFIX: &str = "STUKI1:";

/// The whole project as one line: `STUKI1:` + base64url(deflate(JSON)).
pub fn to_compact(p: &Program) -> String {
    let json = serde_json::to_string(&to_json(p)).unwrap_or_default();
    compact::pack(COMPACT_PREFIX, &json)
}

/// Does this clipboard text hold a structogram (rather than a specification)?
pub fn looks_like_stuki(text: &str) -> bool {
    let t = text.trim();
    t.starts_with(COMPACT_PREFIX)
        || (t.starts_with('{') && t.contains("\"structograms\""))
        || (t.contains("/stuki") && (t.contains("data=") || t.contains("uuid=")))
}

/// Read a structogram from clipboard text: the compact form, `.st` JSON, or a web app link
/// (`https://progalap.elte.hu/stuki/v1/?data=…`).
pub fn from_clipboard(text: &str) -> Result<Program, String> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix(COMPACT_PREFIX) {
        return from_st(&compact::unpack(COMPACT_PREFIX, rest, "structogram")?);
    }
    if t.starts_with('{') {
        return from_st(t);
    }
    if let Some(i) = t.find("data=") {
        let raw = &t[i + 5..];
        let raw = &raw[..raw.find(['&', '#']).unwrap_or(raw.len())];
        let b64: String = percent_decode(raw)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let gz =
            compact::b64_decode(&b64).ok_or("The link's data is damaged (not valid base64).")?;
        let json = gunzip(&gz).ok_or("The link's data is damaged (not valid gzip).")?;
        let json = String::from_utf8(json)
            .map_err(|_| "The link's data is not valid text.".to_string())?;
        return from_st(&json);
    }
    if t.contains("uuid=") {
        return Err("This link points to a structogram saved on the university server, which can't be reached offline. Open it in the web app and use its Share button (a link with data=), or copy the structogram there.".into());
    }
    if t.is_empty() {
        Err("The clipboard is empty.".into())
    } else {
        Err("The clipboard doesn't contain a structogram: expected text starting with STUKI1:, a .st file's contents, or a Stuki link with data=.".into())
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// gzip (what the browser's CompressionStream makes) → raw deflate → bytes.
fn gunzip(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 18 || d[0] != 0x1f || d[1] != 0x8b || d[2] != 8 {
        return None;
    }
    let flags = d[3];
    let mut i = 10;
    if flags & 4 != 0 {
        let n = u16::from_le_bytes([*d.get(i)?, *d.get(i + 1)?]) as usize;
        i += 2 + n;
    }
    for bit in [8u8, 16] {
        if flags & bit != 0 {
            i += d.get(i..)?.iter().position(|&c| c == 0)? + 1;
        }
    }
    if flags & 2 != 0 {
        i += 2;
    }
    compact::inflate(d.get(i..)?)
}

#[cfg(test)]
mod tests {
    use super::super::model::tests::sample;
    use super::*;
    #[test]
    fn damaged_files_are_refused_quickly() {
        let own_body = r#"{"nodes": {
            "s": {"childrenIds": ["w"]},
            "w": {"type": "while", "condition": "", "bodySequenceId": "s"}
        }, "structograms": [{"rootId": "s"}]}"#;
        assert!(from_st(own_body).unwrap_err().contains("nested too deeply"));

        let mut nodes = Map::new();
        for level in 0..60 {
            let next = format!("s{}", level + 1);
            nodes.insert(
                format!("s{level}"),
                json!({"childrenIds": [format!("i{level}")]}),
            );
            nodes.insert(
                format!("i{level}"),
                json!({"type": "if", "condition": "", "trueBranchSequenceId": next, "falseBranchSequenceId": next}),
            );
        }
        let shared = json!({"nodes": nodes, "structograms": [{"rootId": "s0"}]});
        let started = std::time::Instant::now();
        let e = from_json(&shared).unwrap_err();
        assert!(e.contains("too many elements"), "{e}");
        assert!(started.elapsed().as_secs() < 5);
    }

    #[test]
    fn json_round_trip() {
        let p = sample();
        let back = from_st(&to_st(&p)).unwrap();
        assert_eq!(back.content(), p.content());
        let back = from_clipboard(&to_compact(&p)).unwrap();
        assert_eq!(back.content(), p.content());
    }

    #[test]
    fn reads_web_app_state_and_links() {
        let web = r##"{"id":"0.1","hoverId":null,"selectedId":"x","nodes":{
            "r":{"id":"r","parentId":null,"hidden":false,"type":"sequence","childrenIds":["a","p","b"]},
            "a":{"id":"a","parentId":"r","hidden":false,"type":"statement","text":"x:=<span style=\"color: rgb(255, 0, 0);\">1</span>"},
            "p":{"id":"p","parentId":"r","hidden":false,"type":"placeholder"},
            "b":{"id":"b","parentId":"r","hidden":false,"type":"if","condition":"x&gt;0","trueBranchSequenceId":"t","falseBranchSequenceId":"f","backgroundColor":"#ccccff"},
            "t":{"id":"t","parentId":"b","hidden":false,"type":"sequence","childrenIds":[]},
            "f":{"id":"f","parentId":"b","hidden":false,"type":"sequence","childrenIds":["n"]},
            "n":{"id":"n","parentId":"f","hidden":false,"type":"noop"}},
            "structograms":[{"name":"Main","returnText":"","variables":"","rootId":"r","showVariables":false,"showHeader":false,"showReturnText":true}],
            "colors":{"text":["#123456"],"textBackground":[],"nodeBackground":[],"structogramBackground":[]}}"##;
        let p = from_clipboard(web).unwrap();
        let body = &p.diagrams[0].body;
        assert_eq!(body.len(), 2);
        match &body[0].kind {
            Kind::Statement(t) => {
                assert_eq!(t.text, "x:=1");
                assert_eq!(t.styles[3].color, Some(0xff0000));
            }
            _ => panic!(),
        }
        match &body[1].kind {
            Kind::If { cond, yes, .. } => {
                assert_eq!(cond.text, "x>0");
                assert!(yes[0].is_noop());
            }
            _ => panic!(),
        }
        assert_eq!(p.colors.text, vec!["#123456"]);
        // the web app's Share link: gzip + standard base64, URL-encoded
        let gz = {
            let deflated = miniz_oxide::deflate::compress_to_vec(web.as_bytes(), 6);
            let mut g = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3];
            g.extend(deflated);
            g.extend([0u8; 8]);
            g
        };
        let b64 = compact::b64_encode(&gz)
            .replace('-', "%2B")
            .replace('_', "%2F");
        let link = format!("https://progalap.elte.hu/stuki/v1/?data={b64}");
        assert!(looks_like_stuki(&link));
        let q = from_clipboard(&link).unwrap();
        assert_eq!(q.content(), p.content());
        assert!(from_clipboard("https://progalap.elte.hu/stuki/v1/?uuid=1234").is_err());
        assert!(from_clipboard("STUKI1:@@").is_err());
    }

    #[test]
    fn tells_structograms_from_specifications() {
        assert!(looks_like_stuki(" STUKI1:abc"));
        assert!(looks_like_stuki("{\"nodes\":{},\"structograms\":[]}"));
        assert!(looks_like_stuki(
            "https://progalap.elte.hu/stuki/v1/?uuid=1"
        ));
        assert!(!looks_like_stuki("SPECI1:abc"));
        assert!(!looks_like_stuki("Be: n∈N"));
        assert!(!looks_like_stuki("{\"other\": 1}"));
        let e = from_clipboard("{\"structograms\":[]}").unwrap_err();
        assert!(e.contains("\"nodes\" list is missing"), "{e}");
        assert!(from_clipboard("").unwrap_err().contains("empty"));
        assert!(
            from_clipboard("hello")
                .unwrap_err()
                .contains("doesn't contain a structogram")
        );
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("a%2Bb%2fc%3D"), "a+b/c=");
        assert_eq!(percent_decode("%C5%91"), "ő");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz%4"), "%zz%4");
    }

    #[test]
    fn gunzip_skips_the_optional_header_fields() {
        let text = b"structogram".repeat(20);
        let deflated = miniz_oxide::deflate::compress_to_vec(&text, 6);
        let gzip = |flags: u8, extra: &[u8]| {
            let mut g = vec![0x1f, 0x8b, 8, flags, 0, 0, 0, 0, 0, 3];
            g.extend_from_slice(extra);
            g.extend_from_slice(&deflated);
            g.extend([0u8; 8]);
            g
        };
        assert_eq!(gunzip(&gzip(0, &[])).unwrap(), text);
        assert_eq!(gunzip(&gzip(4, &[3, 0, 1, 2, 3])).unwrap(), text);
        assert_eq!(gunzip(&gzip(8, b"name.st\0")).unwrap(), text);
        assert_eq!(gunzip(&gzip(8 | 16 | 2, b"n\0c\0\x12\x34")).unwrap(), text);
        assert!(gunzip(b"plain text, long enough to look at").is_none());
        assert!(gunzip(&gzip(0, &[])[..12]).is_none());
        let mut other = gzip(0, &[]);
        other[2] = 7;
        assert!(gunzip(&other).is_none());
        assert!(gunzip(&gzip(8, b"never ends")).is_none());
    }

    #[test]
    fn damaged_files_are_refused_or_repaired() {
        let looped = r#"{"nodes":{"r":{"type":"sequence","childrenIds":["w"]},
            "w":{"type":"while","condition":"x","bodySequenceId":"r"}},
            "structograms":[{"rootId":"r"}]}"#;
        assert!(from_st(looped).unwrap_err().contains("nested too deeply"));
        let bare = r#"{"nodes":{},"structograms":[{"name":"A"}]}"#;
        let p = from_st(bare).unwrap();
        assert_eq!(p.diagrams[0].name.text, "A");
        assert!(p.diagrams[0].body[0].is_noop() && p.diagrams[0].show_return);
        assert_eq!(p.colors, Palettes::default());
        let sw = r#"{"nodes":{"r":{"type":"sequence","childrenIds":["s","e"]},
            "s":{"type":"switch","cases":[{"type":"default"},{"type":"case","condition":"a"}]},
            "e":{"type":"switch","cases":[]}},
            "structograms":[{"rootId":"r"}]}"#;
        let p = from_st(sw).unwrap();
        assert_eq!(p.diagrams[0].body.len(), 1);
        match &p.diagrams[0].body[0].kind {
            Kind::Switch { cases } => {
                assert_eq!(cases[0].cond.as_ref().unwrap().text, "a");
                assert!(cases[1].cond.is_none());
            }
            _ => panic!("not a switch"),
        }
        assert!(from_st(&format!("\u{feff}{bare}")).is_ok());
    }
}
