use super::launch::create;
use super::*;
use crate::stuki::Stuki;
use crate::ui::{I_COPY, I_DATA, I_SPEC, I_VIEW};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::path::PathBuf;

fn harness(size: [f32; 2]) -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(size)
        .build_eframe(|cc| create(cc, None));
    h.run_steps(8);
    h
}

/// Phones get one pane at a time, picked with tabs.
#[test]
fn narrow_layout_uses_tabs() {
    let mut h = harness([390.0, 800.0]);
    assert!(h.state().narrow);
    h.get_by_label_contains("Tests").click();
    h.run_steps(4);
    assert!(h.state().tab == Tab::Data);
}

/// Wide windows keep all three panes side by side.
#[test]
fn wide_layout_shows_every_pane() {
    let h = harness([1360.0, 900.0]);
    assert!(!h.state().narrow);
    for (icon, title) in [
        (I_SPEC, "Specification"),
        (I_DATA, "Test cases"),
        (I_VIEW, "Evaluation"),
    ] {
        assert!(
            h.query_all_by_label(&format!("{icon}  {title}")).count() >= 1,
            "{title}"
        );
    }
}

/// Stuki: drag an element from the Elements menu into the diagram, drag it out again
/// (deletes it), and undo.
#[test]
fn stuki_drag_and_drop() {
    use crate::stuki::model::Kind;
    let mut h = Harness::builder()
        .with_size([1360.0, 900.0])
        .build_eframe(|cc| {
            let mut a = create(cc, None);
            a.tool = Tool::Stuki;
            a.stuki = Stuki::default();
            a
        });
    h.run_steps(4);
    let drag = |h: &mut Harness<'static, App>, from: egui::Pos2, to: egui::Pos2| {
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        for k in 1..=8 {
            h.hover_at(from + (to - from) * k as f32 / 8.0);
            h.run();
        }
        h.drop_at(to);
        h.run_steps(3);
    };
    let (nodes, elements) = h.state().stuki.seen.clone();
    assert_eq!(elements.len(), 5);
    let noop = nodes[0].1;
    drag(&mut h, elements[1].center(), noop.center()); // decision
    let body = &h.state().stuki.program.diagrams[0].body;
    assert_eq!(body.len(), 1);
    assert!(matches!(body[0].kind, Kind::If { .. }));
    // just below the structogram: after the decision, its last element
    let (nodes, elements) = h.state().stuki.seen.clone();
    let decision = nodes[0].1;
    drag(
        &mut h,
        elements[0].center(),
        decision.center_bottom() + egui::vec2(0.0, 10.0),
    );
    let body = &h.state().stuki.program.diagrams[0].body;
    assert_eq!(body.len(), 2);
    assert!(matches!(body[1].kind, Kind::Statement(_)));
    // the bottom edge of the decision's branch: after the decision, not inside the branch
    let (nodes, elements) = h.state().stuki.seen.clone();
    let branch = nodes[1].1; // the true branch's empty statement
    drag(
        &mut h,
        elements[2].center(),
        branch.center_bottom() - egui::vec2(0.0, 2.0),
    );
    let body = &h.state().stuki.program.diagrams[0].body;
    assert_eq!(body.len(), 3);
    assert!(matches!(body[1].kind, Kind::While { .. }));
    // the middle of the branch's empty statement still replaces it
    let (nodes, elements) = h.state().stuki.seen.clone();
    drag(&mut h, elements[0].center(), nodes[1].1.center());
    match &h.state().stuki.program.diagrams[0].body[0].kind {
        Kind::If { yes, .. } => assert!(matches!(yes[0].kind, Kind::Statement(_))),
        _ => panic!("the decision moved"),
    }
    // drag the decision out of the structogram by its empty corner: deleted
    let (nodes, _) = h.state().stuki.seen.clone();
    let decision = nodes[0].1;
    drag(
        &mut h,
        decision.left_top() + egui::vec2(4.0, 4.0),
        egui::pos2(1300.0, 850.0),
    );
    assert_eq!(h.state().stuki.program.diagrams[0].body.len(), 2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(h.state().stuki.program.diagrams[0].body.len(), 3);
    assert!(h.state().stuki.dirty());
}

/// Stuki: Ctrl+C and the toolbar's copy button copy a picture, Ctrl+Shift+C the text line.
#[test]
fn stuki_copies_pictures() {
    let mut h = Harness::builder()
        .with_size([1360.0, 900.0])
        .build_eframe(|cc| {
            let mut a = create(cc, None);
            a.tool = Tool::Stuki;
            a.stuki = Stuki::default();
            a
        });
    h.run_steps(3);
    let copied = |h: &Harness<'static, App>| {
        h.output()
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyImage(img) => {
                    Some(format!("image {}x{}", img.size[0], img.size[1]))
                }
                egui::OutputCommand::CopyText(t) => Some(t.clone()),
                _ => None,
            })
    };
    h.event(egui::Event::Copy);
    h.step();
    assert!(
        copied(&h).is_some_and(|c| c.starts_with("image")),
        "{:?}",
        copied(&h)
    );
    h.run();
    h.get_by_label(I_COPY).click();
    h.step();
    assert!(
        copied(&h).is_some_and(|c| c.starts_with("image")),
        "{:?}",
        copied(&h)
    );
    h.run();
    h.key_press_modifiers(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::C,
    );
    h.step();
    assert!(
        h.state().stuki.status.contains("whole structogram project"),
        "{}",
        h.state().stuki.status
    );
    // pictures stay light in dark mode
    let img = h.state().stuki.render_png(&h.ctx, None).unwrap();
    assert_eq!(img.pixels[0], egui::Color32::WHITE);
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("speci-local-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn text_positions_round_trip() {
    let text = "Be:\nn∈N,\n\nő";
    for (idx, pos) in [
        (0, (1, 1)),
        (3, (1, 4)),
        (4, (2, 1)),
        (7, (2, 4)),
        (9, (3, 1)),
        (10, (4, 1)),
        (11, (4, 2)),
    ] {
        assert_eq!(line_col(text, idx), pos, "char {idx}");
        assert_eq!(char_index(text, pos.0, pos.1), idx, "{pos:?}");
    }
    assert_eq!(line_col(text, 99), (4, 2));
    assert_eq!(char_index(text, 0, 0), 0);
}

#[test]
fn evaluates_in_the_background() {
    let mut h = harness([1360.0, 900.0]);
    for _ in 0..400 {
        if h.state().evaluated.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.step();
    }
    let app = h.state();
    assert_eq!(app.verdicts(), (1, 0), "{}", app.status);
    assert_eq!(app.status, "1 of 1 test case(s) passed");
    assert!(app.job.is_none() && app.spec_diags.is_empty());
    assert!(app.trace.as_ref().is_some_and(|t| !t.steps.is_empty()));
    assert_eq!(app.evaluated.as_ref(), Some(&app.doc));
}

#[test]
fn unsaved_changes_are_asked_about() {
    let mut h = harness([1360.0, 900.0]);
    assert!(!h.state().dirty());
    h.state_mut().doc.title = "Changed".into();
    assert!(h.state().dirty());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run_steps(3);
    assert!(h.state().confirm.is_some());
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert!(h.state().confirm.is_none());
    assert_eq!(h.state().doc.title, "Changed");

    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run_steps(3);
    h.get_by_label("Discard changes").click();
    h.run_steps(3);
    let app = h.state();
    assert!(app.confirm.is_none() && !app.dirty());
    assert_eq!(app.doc.title, "");
    assert!(app.doc.spec.starts_with("Be:\n\nKi:"));
    assert_eq!(app.doc.tests, [""]);

    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run_steps(3);
    assert!(h.state().confirm.is_none());
}

#[test]
fn opens_and_saves_files() {
    let dir = TempDir::new("files");
    let mut h = harness([1360.0, 900.0]);
    let sp = dir.0.join("Sum of numbers.sp");
    std::fs::write(
        &sp,
        "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n\n/*--- test ---\nn: 1\nx: 1\n---*/\n",
    )
    .unwrap();
    h.state_mut().open_path(sp.clone());
    {
        let app = h.state();
        assert_eq!(app.tool, Tool::Spec);
        assert_eq!(app.path.as_ref(), Some(&sp));
        assert_eq!(app.doc.title, "Sum of numbers");
        assert_eq!(app.doc.tests, ["n: 1\nx: 1\n"]);
        assert!(!app.dirty() && app.status.starts_with("Opened "));
    }
    h.state_mut().doc.tests.push("n: 2\nx: 2\n".into());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(2);
    assert!(!h.state().dirty(), "{}", h.state().status);
    let saved = std::fs::read_to_string(&sp).unwrap();
    assert!(saved.starts_with("// title: Sum of numbers\n") && saved.contains("n: 2\nx: 2\n"));
    h.state_mut().save_to(dir.0.join("copy"));
    assert_eq!(h.state().path, Some(dir.0.join("copy.sp")));
    assert_eq!(
        std::fs::read_to_string(dir.0.join("copy.sp")).unwrap(),
        saved
    );

    let st = dir.0.join("Main.ST");
    std::fs::write(&st, crate::stuki::format::to_st(&Default::default())).unwrap();
    h.state_mut().open_path(st.clone());
    assert_eq!(h.state().tool, Tool::Stuki);
    assert_eq!(h.state().stuki.path.as_ref(), Some(&st));
    assert_eq!(h.state().path, Some(dir.0.join("copy.sp")));

    h.state_mut().open_path(dir.0.join("missing.sp"));
    let app = h.state();
    assert!(app.status.starts_with("Could not open"), "{}", app.status);
    assert_eq!(app.doc.title, "Sum of numbers");
}

#[test]
fn imports_copied_projects() {
    let mut h = harness([1360.0, 900.0]);
    let doc = Doc {
        title: "Pasted".into(),
        spec: "Be: n∈N\nKi: x∈N\nEf: -\nUf: x=n\n".into(),
        tests: vec!["n: 1\nx: 1\n".into()],
    };
    h.state_mut().import_text(&crate::file::to_compact(&doc));
    {
        let app = h.state();
        assert_eq!(app.doc, doc);
        // not saved anywhere yet
        assert!(app.dirty() && app.path.is_none());
        assert!(app.status.contains("read from the clipboard"));
    }
    h.state_mut()
        .import_text("Be: m∈N\nKi: y∈N\nEf: -\nUf: y=m");
    assert!(h.state().confirm.is_some());
    assert_eq!(h.state().doc, doc);
    h.state_mut().confirm = None;

    h.state_mut().import_text("nothing useful");
    assert!(h.state().status.contains("doesn't contain a project"));

    let program = crate::stuki::model::Program::default();
    h.state_mut()
        .import_text(&crate::stuki::format::to_compact(&program));
    assert_eq!(h.state().tool, Tool::Stuki);
    assert!(h.state().stuki.status.contains("read from the clipboard"));
}

#[test]
fn deleting_a_test_keeps_results_aligned() {
    let mut h = harness([1360.0, 900.0]);
    let verdict = |pass| {
        Some(Outcome {
            pass,
            diags: vec![],
            values: vec![],
        })
    };
    {
        let app = h.state_mut();
        app.doc.tests = vec!["a".into(), "b".into(), "c".into()];
        app.results = vec![verdict(true), verdict(false), verdict(true)];
        app.evaluated = Some(app.doc.clone());
        app.current = 1;
        app.delete_test();
        assert_eq!(app.doc.tests, ["a", "c"]);
        assert_eq!(app.verdicts(), (2, 0));
        assert_eq!(app.evaluated.as_ref().unwrap().tests, ["a", "c"]);
        assert_eq!(app.current, 1);
        app.delete_test();
        assert_eq!((app.doc.tests.len(), app.current), (1, 0));
        app.delete_test();
        assert_eq!(app.doc.tests, ["a"]);
    }
}
