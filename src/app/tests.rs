use super::launch::create;
use super::*;
use crate::stuki::Stuki;
use crate::ui::{I_COPY, I_DATA, I_GEAR, I_SPEC, I_STUKI, I_VIEW};
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

#[test]
fn stuki_empty_statement_stays_put() {
    use crate::stuki::model::{Kind, Node, Tag};
    let mut h = Harness::builder()
        .with_size([1360.0, 900.0])
        .build_eframe(|cc| {
            let mut a = create(cc, None);
            a.tool = Tool::Stuki;
            a.stuki = Stuki::default();
            a.stuki.program.diagrams[0].body = vec![Node::new(Tag::While)];
            a
        });
    h.run_steps(4);
    let (nodes, _) = h.state().stuki.seen.clone();
    let (looped, inner) = (nodes[0].1, nodes[1].1);
    for to in [
        looped.center_bottom() + egui::vec2(0.0, 10.0),
        egui::pos2(1300.0, 850.0),
    ] {
        h.hover_at(inner.center());
        h.run();
        h.drag_at(inner.center());
        h.run();
        for k in 1..=8 {
            h.hover_at(inner.center() + (to - inner.center()) * k as f32 / 8.0);
            h.run();
        }
        h.drop_at(to);
        h.run_steps(3);
        let body = &h.state().stuki.program.diagrams[0].body;
        assert_eq!(body.len(), 1);
        assert!(matches!(&body[0].kind, Kind::While { body, .. } if body[0].is_noop()));
    }
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

/// The Konami code (↑↑↓↓←→←→BA) shows the hidden From spec button, or hides it again.
fn konami(h: &mut Harness<'static, App>) {
    use egui::Key::*;
    for k in [
        ArrowUp, ArrowUp, ArrowDown, ArrowDown, ArrowLeft, ArrowRight, ArrowLeft, ArrowRight, B, A,
    ] {
        h.key_press(k);
        h.run_steps(1);
    }
    h.run_steps(2);
}

/// From spec: the button on the Stuki tab (hidden until the Konami code) builds the
/// structograms from the specification (the sample counts odd numbers), and a broken
/// specification leaves them alone.
#[test]
fn structograms_from_the_spec() {
    use crate::stuki::model::Kind;
    use crate::ui::I_WAND;
    let mut h = Harness::builder()
        .with_size([1360.0, 900.0])
        .build_eframe(|cc| {
            let mut a = create(cc, None);
            a.doc = sample();
            a.tool = Tool::Stuki;
            a.stuki = Stuki::default();
            a
        });
    h.run_steps(4);
    let button = format!("{I_WAND}  From spec");
    assert_eq!(h.query_all_by_label(&button).count(), 0, "hidden at first");
    // ↑↑↓↓ alone is not the code; an extra ↑ in front doesn't spoil it
    h.key_press(egui::Key::ArrowUp);
    konami(&mut h);
    h.get_by_label(&button).click();
    h.run_steps(4);
    let st = &h.state().stuki;
    assert_eq!(st.program.diagrams.len(), 1, "{}", st.status);
    let d = &st.program.diagrams[0];
    assert_eq!(d.name.text, "Páratlan számok");
    assert_eq!(d.variables.text, "i:Egész");
    assert!(matches!(&d.body[0].kind, Kind::Statement(t) if t.text == "Be: n, szamok"));
    assert!(matches!(&d.body[1].kind, Kind::Statement(t) if t.text == "db:=0"));
    assert!(matches!(&d.body[2].kind, Kind::While { cond, .. } if cond.text == "i=1..n"));
    assert!(matches!(&d.body[3].kind, Kind::Statement(t) if t.text == "Ki: db"));
    assert!(st.dirty() && st.path.is_none());

    // a broken specification changes nothing
    h.state_mut().doc.spec = "Be:\nn∈N\nKi:\nUf:\n".into();
    h.get_by_label(&format!("{I_WAND}  From spec")).click();
    h.run_steps(4);
    assert!(h.state().stuki.status.starts_with("Fix the specification"));
    assert_eq!(h.state().stuki.program.diagrams[0].body.len(), 4);

    // the code again hides it
    konami(&mut h);
    assert_eq!(h.query_all_by_label(&button).count(), 0);
}

#[test]
fn stuki_runs_the_test_cases() {
    use crate::stuki::model::Kind;
    use crate::ui::{I_FAIL, I_RUN, I_WAND};
    let mut h = Harness::builder()
        .with_size([1360.0, 900.0])
        .build_eframe(|cc| {
            let mut a = create(cc, None);
            a.doc = sample();
            a.doc.tests.push("n: 2\nszamok: [2, 4]\ndb: 0\n".into());
            a.tool = Tool::Stuki;
            a.stuki = Stuki::default();
            a
        });
    h.run_steps(4);
    konami(&mut h);
    h.get_by_label(&format!("{I_WAND}  From spec")).click();
    h.run_steps(4);
    h.get_by_label(&format!("{I_RUN}  Run tests")).click();
    let wait = |h: &mut Harness<'static, App>| {
        h.run_steps(2);
        for _ in 0..400 {
            if let Some(v) = h.state().stuki.verdicts() {
                return v;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            h.step();
        }
        panic!("the run never finished");
    };
    assert_eq!(
        wait(&mut h),
        Ok(vec![(true, String::new()), (true, String::new())])
    );
    h.run_steps(2);
    assert_eq!(h.query_all_by_label("All 2 test case(s) pass").count(), 1);

    let decision = {
        let body = &mut h.state_mut().stuki.program.diagrams[0].body;
        let Kind::While { body, .. } = &mut body[2].kind else {
            panic!("the counting loop")
        };
        let Kind::If { cond, .. } = &mut body[0].kind else {
            panic!("the decision")
        };
        *cond = crate::stuki::rich::Rich::plain("szamok[i+1] mod 2 = 1");
        body[0].id
    };
    let failed = wait(&mut h).unwrap();
    assert!(
        failed
            .iter()
            .all(|(pass, m)| !pass && m.contains("out of range")),
        "{failed:?}"
    );
    h.run_steps(2);
    h.get_by_label(&format!("{I_FAIL} 2")).click();
    h.run_steps(2);
    assert_eq!(h.state().stuki.selected(), Some(decision));

    h.state_mut().doc.spec = "Be:\nn∈N\nKi:\nUf:\n".into();
    let broken = wait(&mut h).unwrap_err();
    assert!(
        broken.starts_with("Fix the specification first"),
        "{broken}"
    );
}

/// The toolbar and the pane headers: no two widgets drawn over each other (issue #8).
#[test]
fn toolbar_widgets_never_overlap() {
    let mut bad = vec![];
    for tool in [Tool::Spec, Tool::Stuki] {
        for font in [12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0] {
            for width in (500..=1900).step_by(40) {
                let mut h = Harness::builder()
                    .with_size([width as f32, 900.0])
                    .build_eframe(move |cc| {
                        let mut a = create(cc, None);
                        a.font = font;
                        a.tool = tool;
                        a
                    });
                h.run_steps(6);
                if width >= 1900 {
                    // wide windows keep the buttons on the row of the tabs
                    let tab = h.get_by_label(&format!("{I_STUKI}  Stuki")).rect();
                    let gear = h.get_by_label(I_GEAR).rect();
                    assert!(
                        (tab.center().y - gear.center().y).abs() < 2.0,
                        "{tool:?} font {font}"
                    );
                }
                let rows = font * 8.0; // the toolbar (one or two rows) and the pane headers
                let mut rects = vec![];
                leaves(h.root(), egui::Rect::EVERYTHING, &mut rects);
                rects.retain(|(_, r)| r.min.y < rows && r.width() > 0.0);
                for (i, (a, ra)) in rects.iter().enumerate() {
                    for (b, rb) in &rects[i + 1..] {
                        let x = ra.intersect(*rb);
                        if x.width() > 1.0 && x.height() > 1.0 {
                            bad.push(format!("{tool:?} font {font} width {width}: {a} / {b}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} overlaps:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// The widgets with nothing inside them, as (role and label, rect). Single-line text fields
/// clip their text, so the text counts only as far as the field reaches.
fn leaves(n: egui_kittest::Node<'_>, clip: egui::Rect, out: &mut Vec<(String, egui::Rect)>) {
    use egui_kittest::kittest::NodeT;
    let role = n.accesskit_node().role();
    let clip = if role == egui::accesskit::Role::TextInput {
        clip.intersect(n.rect())
    } else {
        clip
    };
    if n.children().next().is_none() {
        let label = n.accesskit_node().label().unwrap_or_default();
        out.push((format!("{role:?} {label:?}"), n.rect().intersect(clip)));
    }
    for c in n.children() {
        leaves(c, clip, out);
    }
}
