use super::model::{Diagram, Kind, Node, Place, Tag};
use super::rich::Rich;
use super::settings::Settings;
use super::*;
use std::collections::HashSet;

/// The layout and the PNG renderer work on every kind of element, with every setting on.
#[test]
fn renders_png() {
    let ctx = egui::Context::default();
    crate::app::fonts(&ctx);
    let mut st = Stuki::default();
    let mut last = st.program.diagrams[0].body[0].id;
    for tag in Tag::ALL {
        last = st.program.add_next_to(last, tag, true).unwrap();
    }
    // texts, a nested element and some colors
    for d in &mut st.program.diagrams {
        for n in &mut d.body {
            match &mut n.kind {
                Kind::Statement(t) => {
                    *t = Rich::plain("s:=s+x[i]");
                    t.paint(5, 9, |s| s.color = Some(0xff0000));
                }
                Kind::If { cond, yes, .. } => {
                    *cond = Rich::plain("x[i] mod 2 = 1");
                    yes[0] = Node::new(Tag::Statement);
                    if let Kind::Statement(t) = &mut yes[0].kind {
                        *t = Rich::plain("db:=db+1");
                    }
                }
                Kind::While { cond, .. } => *cond = Rich::plain("i≤n"),
                Kind::Repeat { cond, .. } => *cond = Rich::plain("i>n"),
                Kind::Switch { cases } => {
                    if let Some(c) = &mut cases[0].cond {
                        *c = Rich::plain("a<0");
                    }
                }
                Kind::Noop => {}
            }
        }
        d.body[1].bg = Some("#ccccff".into());
    }
    let d = &mut st.program.diagrams[0];
    d.show_header = true;
    d.show_variables = true;
    d.name = Rich::plain("Main(n)");
    st.settings = Settings {
        hdist: 3,
        vdist: 3,
        spacing: 2,
        block_height: 4,
        right_padding: true,
        ..Settings::default()
    };
    let mut img = None;
    let mut out = ctx.run_ui(Default::default(), |ui| {
        img = st.render_png(ui.ctx(), None);
    });
    out.textures_delta.clear();
    let img = img.unwrap();
    assert!(img.size[0] > 100 && img.size[1] > 100, "{:?}", img.size);
    // there is something besides the background: lines, colors and text
    let distinct: HashSet<_> = img.pixels.iter().map(|c| c.to_array()).collect();
    assert!(distinct.len() > 20);
    let bytes = png::encode(&img).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
}

fn body_len(st: &Stuki) -> usize {
    st.program.diagrams[0].body.len()
}

#[test]
fn undo_and_redo_step_through_the_edits() {
    let mut st = Stuki::default();
    assert!(!st.dirty());
    let first = st.program.diagrams[0].body[0].id;
    // an empty statement is replaced, anything else gets a neighbour
    st.push_undo();
    let a = st.program.add_next_to(first, Tag::While, true).unwrap();
    st.push_undo();
    st.program.add_next_to(a, Tag::Statement, false).unwrap();
    assert_eq!(body_len(&st), 2);
    assert!(st.dirty());
    st.undo();
    assert_eq!(body_len(&st), 1);
    assert!(matches!(
        st.program.diagrams[0].body[0].kind,
        Kind::While { .. }
    ));
    st.undo();
    assert!(st.program.diagrams[0].body[0].is_noop());
    assert!(!st.dirty(), "back at the saved state");
    st.undo();
    assert!(st.program.diagrams[0].body[0].is_noop());
    st.redo();
    st.redo();
    assert_eq!(body_len(&st), 2);
    st.undo();
    st.push_undo();
    st.program.diagrams.push(Diagram::new());
    st.redo();
    assert_eq!((st.program.diagrams.len(), body_len(&st)), (2, 1));
    for _ in 0..UNDO_LIMIT + 50 {
        st.push_undo();
    }
    assert_eq!(st.undo.len(), UNDO_LIMIT);
}

#[test]
fn actions_edit_the_program_and_can_be_undone() {
    let mut st = Stuki::default();
    let d = st.program.diagrams[0].id;
    let noop = st.program.diagrams[0].body[0].id;
    st.apply(Action::Drop(
        Drag::New(Tag::Switch),
        Target::At(noop, Place::Replace, Rect::NOTHING),
    ));
    let sw = st.selected.expect("the new element is selected");
    let cases = |st: &Stuki| match &st.program.find(sw).unwrap().kind {
        Kind::Switch { cases } => cases.iter().map(|c| c.cond.is_some()).collect::<Vec<_>>(),
        _ => panic!("not a switch"),
    };
    assert_eq!(cases(&st), [true, true, false]);
    st.apply(Action::AddCase(sw, false));
    st.apply(Action::AddCase(sw, true));
    assert_eq!(cases(&st), [true, true, true, false]);
    st.undo();
    st.undo();
    assert_eq!(cases(&st), [true, true, false]);

    st.apply(Action::Toggle(d, Part::Header));
    st.apply(Action::Toggle(d, Part::Variables));
    st.apply(Action::Toggle(d, Part::Return));
    let dia = &st.program.diagrams[0];
    assert_eq!(
        (dia.show_header, dia.show_variables, dia.show_return),
        (true, true, false)
    );

    st.apply(Action::AddDiagram);
    assert_eq!(st.program.diagrams.len(), 2);
    st.apply(Action::DelDiagram(d));
    assert_eq!(st.program.diagrams.len(), 1);
    assert!(st.program.find(sw).is_none());
    st.undo();
    assert!(st.program.find(sw).is_some());

    st.apply(Action::Drop(Drag::Node(sw), Target::Delete));
    assert!(st.program.find(sw).is_none() && st.selected.is_none());
    assert!(st.program.diagrams[0].body[0].is_noop());
    assert!(st.status.starts_with("Deleted"));
    let steps = st.undo.len();
    st.apply(Action::Drop(Drag::New(Tag::If), Target::Nothing));
    assert_eq!(st.undo.len(), steps);
}

#[test]
fn saves_and_loads_st_files() {
    let dir = std::env::temp_dir().join(format!("speci-local-stuki-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut st = Stuki::default();
    assert_eq!(st.suggested_name(), "Main");
    st.program.diagrams[0].name = Rich::plain("a/b: c?");
    assert_eq!(st.suggested_name(), "a_b_ c_");
    st.program.diagrams[0].name = Rich::plain("  ");
    assert_eq!(st.suggested_name(), "structogram");
    assert!(st.dirty());

    st.save_to(dir.join("sum"));
    let path = dir.join("sum.st");
    assert_eq!(st.path.as_ref(), Some(&path));
    assert!(!st.dirty() && st.status.starts_with("Saved "));
    assert_eq!(st.file_name(), "sum.st");
    assert_eq!(st.suggested_name(), "sum");

    let first = st.program.diagrams[0].body[0].id;
    st.program.add_next_to(first, Tag::If, true).unwrap();
    assert!(st.dirty());
    st.request(Pending::Load(path.clone()));
    assert!(st.confirm.is_some());
    st.confirm = None;
    st.load(&path);
    assert!(!st.dirty() && st.undo.is_empty());
    assert!(st.program.diagrams[0].body[0].is_noop());

    std::fs::write(&path, "not json").unwrap();
    st.load(&path);
    assert!(
        st.status.contains("not a structogram file"),
        "{}",
        st.status
    );
    st.request(Pending::New);
    assert!(st.path.is_none() && st.status == "New structogram");
    let _ = std::fs::remove_dir_all(&dir);
}
