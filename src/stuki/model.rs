//! The structogram program: a list of diagrams, each a tree of nodes, plus the color palettes.
//!
//! Internally a plain tree (easy to draw and edit). Files use the web Stuki's own state format
//! (a flat `nodes` map plus `structograms`, see [`to_json`]), so `.st` files and the web app's
//! `?data=` links can be moved between the two.

use super::rich::Rich;
use serde::{Deserialize, Serialize};
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

impl Kind {
    pub fn tag(&self) -> Tag {
        match self {
            Kind::Noop | Kind::Statement(_) => Tag::Statement,
            Kind::If { .. } => Tag::If,
            Kind::While { .. } => Tag::While,
            Kind::Repeat { .. } => Tag::Repeat,
            Kind::Switch { .. } => Tag::Switch,
        }
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

    pub fn content_is(&self, text: &str) -> bool {
        struct Expect<'a>(&'a [u8]);
        impl std::io::Write for Expect<'_> {
            fn write(&mut self, part: &[u8]) -> std::io::Result<usize> {
                self.0 = self
                    .0
                    .strip_prefix(part)
                    .ok_or(std::io::ErrorKind::InvalidData)?;
                Ok(part.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut rest = Expect(text.as_bytes());
        serde_json::to_writer(&mut rest, self).is_ok() && rest.0.is_empty()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) fn sample() -> Program {
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
    fn content_is_compared_without_building_it() {
        let mut p = sample();
        let saved = p.content();
        assert!(p.content_is(&saved));
        assert!(!p.content_is(&saved[..saved.len() - 1]));
        assert!(!p.content_is(&format!("{saved} ")));
        assert!(!p.content_is(""));
        p.diagrams[0].variables = Rich::plain("j:Egész");
        assert!(!p.content_is(&saved));
        assert!(p.content_is(&p.content()));
        let copy: Program = serde_json::from_str(&saved).unwrap();
        assert!(copy.content_is(&saved));
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
}
