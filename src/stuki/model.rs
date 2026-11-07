//! The structogram program: a list of diagrams, each a tree of nodes, plus the color palettes.
//!
//! Internally a plain tree (easy to draw and edit). Files use the web Stuki's own state format
//! (a flat `nodes` map plus `structograms`, see [`to_json`]), so `.st` files and the web app's
//! `?data=` links can be moved between the two.

use super::rich::Rich;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A process-wide unique id for nodes, switch cases and diagrams (never saved: files get their own).
pub fn fresh_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// The kinds of elements, as named in the web app's files.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tag {
    Statement,
    If,
    While,
    Repeat,
    Switch,
}

impl Tag {
    pub const ALL: [Tag; 5] = [
        Tag::Statement,
        Tag::If,
        Tag::While,
        Tag::Repeat,
        Tag::Switch,
    ];

    pub fn description(self) -> &'static str {
        match self {
            Tag::Statement => "statement",
            Tag::If => "decision",
            Tag::While => "pre-test loop",
            Tag::Repeat => "post-test loop",
            Tag::Switch => "multi-way decision",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    #[serde(skip, default = "fresh_id")]
    pub id: u64,
    /// Node background color (`#rrggbb`), mixed into the element's own color.
    #[serde(default)]
    pub bg: Option<String>,
    pub kind: Kind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    /// The empty statement `-` (the only content of an empty block).
    Noop,
    Statement(Rich),
    If {
        cond: Rich,
        yes: Vec<Node>,
        no: Vec<Node>,
    },
    While {
        cond: Rich,
        body: Vec<Node>,
    },
    Repeat {
        cond: Rich,
        body: Vec<Node>,
    },
    Switch {
        cases: Vec<Case>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Case {
    #[serde(skip, default = "fresh_id")]
    pub id: u64,
    /// `None`: the default case (always the last one).
    pub cond: Option<Rich>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagram {
    #[serde(skip, default = "fresh_id")]
    pub id: u64,
    pub name: Rich,
    pub variables: Rich,
    pub ret: Rich,
    pub show_header: bool,
    pub show_variables: bool,
    pub show_return: bool,
    #[serde(default)]
    pub bg: Option<String>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Palettes {
    pub text: Vec<String>,
    pub text_background: Vec<String>,
    pub node_background: Vec<String>,
    pub structogram_background: Vec<String>,
}

impl Default for Palettes {
    fn default() -> Self {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect();
        Palettes {
            text: v(&[
                "#ff0000", "#0000ff", "#ffc000", "#00b050", "#00b0f0", "#7030a0", "#ceaf87",
            ]),
            text_background: v(&[
                "#ffb2b2", "#b2b2ff", "#ffecb2", "#b2e7ca", "#b2e7fa", "#d4c0e2",
            ]),
            node_background: v(&[
                "#ffcccc", "#ccccff", "#fff2cc", "#ccefdc", "#b2e7fa", "#d4c0e2",
            ]),
            structogram_background: v(&[
                "#ffffff", "#ffcccc", "#ccccff", "#fff2cc", "#ccefdc", "#ff99ff", "#008c95",
                "#eee4d6", "#ceaf87",
            ]),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Program {
    pub diagrams: Vec<Diagram>,
    #[serde(default)]
    pub colors: Palettes,
}

impl Default for Program {
    /// What the web app starts with: one diagram called "Main" holding an empty statement.
    fn default() -> Self {
        let mut d = Diagram::new();
        d.name = Rich::plain("Main");
        Program {
            diagrams: vec![d],
            colors: Palettes::default(),
        }
    }
}

impl Diagram {
    pub fn new() -> Self {
        Diagram {
            id: fresh_id(),
            name: Rich::default(),
            variables: Rich::default(),
            ret: Rich::default(),
            show_header: false,
            show_variables: false,
            show_return: true,
            bg: None,
            body: vec![Node::noop()],
        }
    }
}

impl Default for Diagram {
    fn default() -> Self {
        Self::new()
    }
}

impl Node {
    pub fn noop() -> Node {
        Node {
            id: fresh_id(),
            bg: None,
            kind: Kind::Noop,
        }
    }

    /// A new, empty element of the given kind (what the Elements menu inserts).
    pub fn new(tag: Tag) -> Node {
        let seq = || vec![Node::noop()];
        let kind = match tag {
            Tag::Statement => Kind::Statement(Rich::default()),
            Tag::If => Kind::If {
                cond: Rich::default(),
                yes: seq(),
                no: seq(),
            },
            Tag::While => Kind::While {
                cond: Rich::default(),
                body: seq(),
            },
            Tag::Repeat => Kind::Repeat {
                cond: Rich::default(),
                body: seq(),
            },
            Tag::Switch => Kind::Switch {
                cases: vec![Case::new(true), Case::new(true), Case::new(false)],
            },
        };
        Node {
            id: fresh_id(),
            bg: None,
            kind,
        }
    }

    pub fn is_noop(&self) -> bool {
        matches!(self.kind, Kind::Noop)
    }

    pub fn seqs(&self) -> Vec<&Vec<Node>> {
        match &self.kind {
            Kind::Noop | Kind::Statement(_) => vec![],
            Kind::If { yes, no, .. } => vec![yes, no],
            Kind::While { body, .. } | Kind::Repeat { body, .. } => vec![body],
            Kind::Switch { cases } => cases.iter().map(|c| &c.body).collect(),
        }
    }

    pub fn seqs_mut(&mut self) -> Vec<&mut Vec<Node>> {
        match &mut self.kind {
            Kind::Noop | Kind::Statement(_) => vec![],
            Kind::If { yes, no, .. } => vec![yes, no],
            Kind::While { body, .. } | Kind::Repeat { body, .. } => vec![body],
            Kind::Switch { cases } => cases.iter_mut().map(|c| &mut c.body).collect(),
        }
    }

    /// Is `id` this node or one of its descendants?
    pub fn contains(&self, id: u64) -> bool {
        self.id == id || self.seqs().iter().any(|s| s.iter().any(|n| n.contains(id)))
    }
}

impl Case {
    pub fn new(normal: bool) -> Case {
        Case {
            id: fresh_id(),
            cond: normal.then(Rich::default),
            body: vec![Node::noop()],
        }
    }
}

/// Where to put an element relative to another one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Before,
    After,
    /// Instead of the target (only used for empty statements).
    Replace,
}

/// Which editable text: a node's statement / condition, a case condition, or a diagram's texts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TextKey {
    Node(u64),
    Case(u64),
    Name(u64),
    Vars(u64),
    Ret(u64),
}

fn seq_of(seq: &mut Vec<Node>, id: u64) -> Option<(&mut Vec<Node>, usize)> {
    if let Some(i) = seq.iter().position(|n| n.id == id) {
        return Some((seq, i));
    }
    for n in seq.iter_mut() {
        for s in n.seqs_mut() {
            if let Some(r) = seq_of(s, id) {
                return Some(r);
            }
        }
    }
    None
}

fn find_in(seq: &[Node], id: u64) -> Option<&Node> {
    for n in seq {
        if n.id == id {
            return Some(n);
        }
        for s in n.seqs() {
            if let Some(r) = find_in(s, id) {
                return Some(r);
            }
        }
    }
    None
}

fn find_in_mut(seq: &mut [Node], id: u64) -> Option<&mut Node> {
    for n in seq.iter_mut() {
        if n.id == id {
            return Some(n);
        }
        for s in n.seqs_mut() {
            if let Some(r) = find_in_mut(s, id) {
                return Some(r);
            }
        }
    }
    None
}

impl Program {
    /// The sequence holding node `id`, and its index there.
    fn seq_of(&mut self, id: u64) -> Option<(&mut Vec<Node>, usize)> {
        self.diagrams
            .iter_mut()
            .find_map(|d| seq_of(&mut d.body, id))
    }

    pub fn find(&self, id: u64) -> Option<&Node> {
        self.diagrams.iter().find_map(|d| find_in(&d.body, id))
    }

    pub fn find_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.diagrams
            .iter_mut()
            .find_map(|d| find_in_mut(&mut d.body, id))
    }

    /// Index of the diagram that holds node `id`.
    pub fn diagram_of(&self, id: u64) -> Option<usize> {
        self.diagrams
            .iter()
            .position(|d| d.body.iter().any(|n| n.contains(id)))
    }

    /// Take node `id` out of its block (an emptied block gets an empty statement).
    pub fn remove(&mut self, id: u64) -> Option<Node> {
        let (seq, i) = self.seq_of(id)?;
        let n = seq.remove(i);
        if seq.is_empty() {
            seq.push(Node::noop());
        }
        Some(n)
    }

    /// Put `node` before / after / instead of `target`. Gives the node back if `target` is gone.
    #[allow(clippy::result_large_err)]
    pub fn insert(&mut self, target: u64, place: Place, node: Node) -> Result<(), Node> {
        let Some((seq, i)) = self.seq_of(target) else {
            return Err(node);
        };
        match place {
            Place::Before => seq.insert(i, node),
            Place::After => seq.insert(i + 1, node),
            Place::Replace => seq[i] = node,
        }
        Ok(())
    }

    /// Drag & drop of an existing node. False if nothing changed (e.g. dropped onto itself).
    pub fn move_node(&mut self, id: u64, target: u64, place: Place) -> bool {
        match self.find(id) {
            Some(n) if !n.contains(target) => {}
            _ => return false,
        }
        let Some(node) = self.remove(id) else {
            return false;
        };
        match self.insert(target, place, node) {
            Ok(()) => true,
            Err(_) => false, // can't happen: the target isn't inside the moved node
        }
    }

    /// The web app's ↑ / ↓ buttons: an empty statement is replaced, anything else gets a neighbour.
    pub fn add_next_to(&mut self, target: u64, tag: Tag, after: bool) -> Option<u64> {
        let node = Node::new(tag);
        let id = node.id;
        let place = match self.find(target)? {
            n if n.is_noop() => Place::Replace,
            _ if after => Place::After,
            _ => Place::Before,
        };
        self.insert(target, place, node).ok()?;
        Some(id)
    }

    /// Add a case (before the default one) or the default case to switch `id`.
    pub fn add_case(&mut self, id: u64, default: bool) {
        if let Some(Node {
            kind: Kind::Switch { cases },
            ..
        }) = self.find_mut(id)
        {
            let has_default = cases.last().is_some_and(|c| c.cond.is_none());
            if default {
                if !has_default {
                    cases.push(Case::new(false));
                }
            } else {
                let at = cases.len() - usize::from(has_default);
                cases.insert(at, Case::new(true));
            }
        }
    }

    /// Delete one case of switch `id`. A switch left with a single case is removed altogether.
    pub fn delete_case(&mut self, id: u64, case: u64) {
        let gone = match self.find_mut(id) {
            Some(Node {
                kind: Kind::Switch { cases },
                ..
            }) => {
                if cases.len() <= 2 {
                    true
                } else {
                    cases.retain(|c| c.id != case);
                    false
                }
            }
            _ => false,
        };
        if gone {
            self.remove(id);
        }
    }

    /// The switch that owns case `case`.
    pub fn switch_of_case(&self, case: u64) -> Option<u64> {
        fn walk(seq: &[Node], case: u64) -> Option<u64> {
            for n in seq {
                if let Kind::Switch { cases } = &n.kind
                    && cases.iter().any(|c| c.id == case)
                {
                    return Some(n.id);
                }
                for s in n.seqs() {
                    if let Some(r) = walk(s, case) {
                        return Some(r);
                    }
                }
            }
            None
        }
        self.diagrams.iter().find_map(|d| walk(&d.body, case))
    }

    pub fn text(&self, key: TextKey) -> Option<&Rich> {
        match key {
            TextKey::Name(d) => self.diagrams.iter().find(|x| x.id == d).map(|x| &x.name),
            TextKey::Vars(d) => self
                .diagrams
                .iter()
                .find(|x| x.id == d)
                .map(|x| &x.variables),
            TextKey::Ret(d) => self.diagrams.iter().find(|x| x.id == d).map(|x| &x.ret),
            TextKey::Node(id) => match &self.find(id)?.kind {
                Kind::Statement(t) => Some(t),
                Kind::If { cond, .. } | Kind::While { cond, .. } | Kind::Repeat { cond, .. } => {
                    Some(cond)
                }
                _ => None,
            },
            TextKey::Case(c) => {
                let sw = self.switch_of_case(c)?;
                match &self.find(sw)?.kind {
                    Kind::Switch { cases } => cases.iter().find(|x| x.id == c)?.cond.as_ref(),
                    _ => None,
                }
            }
        }
    }

    pub fn text_mut(&mut self, key: TextKey) -> Option<&mut Rich> {
        match key {
            TextKey::Name(d) => self
                .diagrams
                .iter_mut()
                .find(|x| x.id == d)
                .map(|x| &mut x.name),
            TextKey::Vars(d) => self
                .diagrams
                .iter_mut()
                .find(|x| x.id == d)
                .map(|x| &mut x.variables),
            TextKey::Ret(d) => self
                .diagrams
                .iter_mut()
                .find(|x| x.id == d)
                .map(|x| &mut x.ret),
            TextKey::Node(id) => match &mut self.find_mut(id)?.kind {
                Kind::Statement(t) => Some(t),
                Kind::If { cond, .. } | Kind::While { cond, .. } | Kind::Repeat { cond, .. } => {
                    Some(cond)
                }
                _ => None,
            },
            TextKey::Case(c) => {
                let sw = self.switch_of_case(c)?;
                match &mut self.find_mut(sw)?.kind {
                    Kind::Switch { cases } => cases.iter_mut().find(|x| x.id == c)?.cond.as_mut(),
                    _ => None,
                }
            }
        }
    }

    /// The node a text belongs to (for selecting it when its text is clicked).
    pub fn owner(&self, key: TextKey) -> Option<u64> {
        match key {
            TextKey::Node(id) => Some(id),
            TextKey::Case(c) => self.switch_of_case(c),
            _ => None,
        }
    }

    /// Everything that matters for "unsaved changes" (ids left out).
    pub fn content(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ---------------------------------------------------------------- web app format

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

    fn seq(
        nodes: &Map<String, Value>,
        id: Option<&str>,
        depth: usize,
    ) -> Result<Vec<Node>, String> {
        if depth > 200 {
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
            if let Some(n) = node(nodes, n, depth + 1)? {
                out.push(n);
            }
        }
        if out.is_empty() {
            out.push(Node::noop());
        }
        Ok(out)
    }
    fn node(nodes: &Map<String, Value>, n: &Value, depth: usize) -> Result<Option<Node>, String> {
        let t = |k: &str| Rich::from_html(n.get(k).and_then(Value::as_str).unwrap_or(""));
        let sub = |k: &str| seq(nodes, n.get(k).and_then(Value::as_str), depth);
        let kind = match n.get("type").and_then(Value::as_str).unwrap_or("") {
            "noop" => Kind::Noop,
            "statement" => Kind::Statement(t("text")),
            "if" => Kind::If {
                cond: t("condition"),
                yes: sub("trueBranchSequenceId")?,
                no: sub("falseBranchSequenceId")?,
            },
            "while" => Kind::While {
                cond: t("condition"),
                body: sub("bodySequenceId")?,
            },
            "repeat" => Kind::Repeat {
                cond: t("condition"),
                body: sub("bodySequenceId")?,
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
                        body: seq(
                            nodes,
                            c.get("bodySequenceId").and_then(Value::as_str),
                            depth,
                        )?,
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
            body: seq(nodes, s.get("rootId").and_then(Value::as_str), 0)?,
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
    let packed = miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 9);
    format!("{COMPACT_PREFIX}{}", crate::file::b64_encode(&packed))
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
        let rest: String = rest.chars().filter(|c| !c.is_whitespace()).collect();
        let packed = crate::file::b64_decode(&rest).ok_or(
            "The text after STUKI1: is damaged (not valid base64). Was it copied completely?",
        )?;
        let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, 16 << 20).map_err(
            |_| {
                "The copied structogram is damaged or incomplete. Copy it again from the source."
                    .to_string()
            },
        )?;
        let json = String::from_utf8(raw)
            .map_err(|_| "The copied structogram is not valid text.".to_string())?;
        return from_st(&json);
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
        let gz = crate::file::b64_decode(&b64)
            .ok_or("The link's data is damaged (not valid base64).")?;
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
    miniz_oxide::inflate::decompress_to_vec_with_limit(d.get(i..)?, 16 << 20).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Program {
        let mut p = Program::default();
        let d = &mut p.diagrams[0];
        d.show_header = true;
        d.variables = Rich::plain("i:Egész");
        let first = d.body[0].id;
        let st = p.add_next_to(first, Tag::Statement, true).unwrap();
        let mut w = Node::new(Tag::While);
        if let Kind::While { cond, .. } = &mut w.kind {
            *cond = Rich::plain("i<n");
        }
        w.bg = Some("#ffcccc".into());
        p.insert(st, Place::Before, w).unwrap();
        let sw = Node::new(Tag::Switch);
        let sid = sw.id;
        p.diagrams[0].body.push(sw);
        p.add_case(sid, false);
        p
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
    fn edit_operations() {
        let mut p = sample();
        let ids: Vec<u64> = p.diagrams[0].body.iter().map(|n| n.id).collect();
        assert_eq!(ids.len(), 3);
        // a node can't be dropped into itself
        let inner = p.diagrams[0].body[0].seqs()[0][0].id;
        assert!(!p.move_node(ids[0], inner, Place::After));
        assert!(p.move_node(ids[2], ids[0], Place::Before));
        assert_eq!(p.diagrams[0].body[0].id, ids[2]);
        // switch: 3 normal cases + default; deleting down to 2 then removes the switch
        let cases: Vec<u64> = match &p.find(ids[2]).unwrap().kind {
            Kind::Switch { cases } => cases.iter().map(|c| c.id).collect(),
            _ => unreachable!(),
        };
        assert_eq!(cases.len(), 4);
        assert!(p.text(TextKey::Case(cases[3])).is_none()); // default
        p.delete_case(ids[2], cases[0]);
        p.delete_case(ids[2], cases[1]);
        p.delete_case(ids[2], cases[2]);
        assert!(p.find(ids[2]).is_none());
        // removing everything leaves an empty statement
        for id in [ids[0], ids[1]] {
            p.remove(id);
        }
        assert!(p.diagrams[0].body.len() == 1 && p.diagrams[0].body[0].is_noop());
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
        let b64 = crate::file::b64_encode(&gz)
            .replace('-', "%2B")
            .replace('_', "%2F");
        let link = format!("https://progalap.elte.hu/stuki/v1/?data={b64}");
        assert!(looks_like_stuki(&link));
        let q = from_clipboard(&link).unwrap();
        assert_eq!(q.content(), p.content());
        assert!(from_clipboard("https://progalap.elte.hu/stuki/v1/?uuid=1234").is_err());
        assert!(from_clipboard("STUKI1:@@").is_err());
    }
}
