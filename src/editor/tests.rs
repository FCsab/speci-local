use super::*;
use egui_kittest::Harness;

struct S {
    text: String,
    comp: Completion,
    first: bool,
}

fn harness(text: &str) -> Harness<'static, S> {
    let id = Id::unique("t-editor");
    let symbols = scan_symbols("Be: szamok∈N[1..n], n∈N\nFv: kisebb:N->L, kisebb(i)=i<3");
    Harness::builder()
        .with_context_setup(crate::app::fonts)
        .build_ui_state(
            move |ui, s: &mut S| {
                if s.first {
                    s.first = false;
                    let n = s.text.chars().count();
                    set_cursor(ui.ctx(), id, n);
                }
                code_editor(
                    ui,
                    id,
                    &mut s.text,
                    Lang::Spec,
                    &symbols,
                    &[],
                    &mut s.comp,
                    "",
                );
            },
            S {
                text: text.into(),
                comp: Completion::default(),
                first: true,
            },
        )
}

#[test]
fn completes_macro_snippet_with_tab() {
    let mut h = harness("Uf: db=");
    h.run_steps(3);
    h.event(egui::Event::Text("DA".into()));
    h.run_steps(3);
    assert!(
        h.state().comp.items.iter().any(|i| i.label == "DARAB"),
        "{:?}",
        h.state().comp.items
    );
    h.key_press(Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().text, "Uf: db=DARAB(i=1..n, )");
    assert!(h.state().comp.items.is_empty());
}

#[test]
fn completes_declared_names_and_navigates() {
    let mut h = harness("Uf: x=");
    h.run_steps(3);
    h.event(egui::Event::Text("ki".into()));
    h.run_steps(3);
    let labels: Vec<_> = h
        .state()
        .comp
        .items
        .iter()
        .map(|i| i.label.clone())
        .collect();
    assert_eq!(labels, ["kisebb"]);
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!(h.state().text, "Uf: x=kisebb()");

    let mut h = harness("Uf: ");
    h.run_steps(3);
    h.event(egui::Event::Text("sz".into()));
    h.run_steps(3);
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(h.state().comp.items.is_empty());
    assert_eq!(h.state().text, "Uf: sz");
}

#[test]
fn click_on_suggestion_inserts_it() {
    use egui_kittest::kittest::Queryable;
    let mut h = harness("Uf: x=");
    h.run_steps(3);
    h.event(egui::Event::Text("SE".into()));
    h.run_steps(3);
    h.get_by_label_contains("SEARCH").click();
    h.run_steps(4);
    assert_eq!(h.state().text, "Uf: x=SEARCH(i=1..n, )");
}

#[test]
fn header_and_type_context() {
    let mut h = harness("Be: n∈N\n");
    h.run_steps(3);
    h.event(egui::Event::Text("U".into()));
    h.run_steps(3);
    assert_eq!(
        h.state().comp.items.first().map(|i| i.label.as_str()),
        Some("Uf:")
    );
    let mut h = harness("Be: x∈");
    h.run_steps(3);
    h.event(egui::Event::Text("z".into()));
    h.run_steps(3);
    assert_eq!(
        h.state().comp.items.first().map(|i| i.detail.as_str()),
        Some("integers")
    );
}

#[test]
fn no_completion_in_comments_or_text() {
    for start in ["Uf: x=1 // ", "Uf: /* ", "Uf: x=\""] {
        let mut h = harness(start);
        h.run_steps(3);
        h.event(egui::Event::Text("SU".into()));
        h.run_steps(3);
        assert!(h.state().comp.items.is_empty(), "{start}");
    }
}

#[test]
fn tab_inserts_spaces() {
    let mut h = harness("n:");
    h.run_steps(3);
    h.key_press(Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().text, "n:  ");
}

#[test]
fn fuzz_highlight_and_scan() {
    let p = palette(false);
    let f = FontId::monospace(14.0);
    let base: Vec<char> = "Be:\nn∈N, t∈Pont[1..n], Pont=(x:Z x y:Z) // é\nFv: f:N->N, f(i)=i\nUf: ∀i∈[1..n]:(t[i].x>0) /* ő */ \"s\" -1.5 a->b\n- name: \"x\" # c\n".chars().collect();
    let mut seed: u64 = 7;
    for _ in 0..3000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let a = (seed >> 33) as usize % base.len();
        let b = (seed >> 13) as usize % base.len();
        let t: String = base[a.min(b)..a.max(b)].iter().collect();
        assert_eq!(highlight_spec(&t, f.clone(), &p, &[2]).text, t);
        assert_eq!(highlight_data(&t, f.clone(), &p, &[2]).text, t);
        let _ = scan_symbols(&t);
    }
}

#[test]
fn alt_digits_insert_symbols() {
    let mut h = harness("Uf: x");
    h.run_steps(3);
    for k in [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
    ] {
        h.key_press_modifiers(Modifiers::ALT, k);
        h.run_steps(2);
    }
    assert_eq!(h.state().text, "Uf: x∈∀∃≠≤≥");
}

#[test]
fn symbol_scan() {
    let s = scan_symbols(
        "Be: n∈N, t∈Pont[1..n], Pont=(x:Z x y:Z)\nFv: f:N->N, f(i)=i\nUf: ∀i∈[1..n]:(t[i].x>0)",
    );
    let l: Vec<_> = s.iter().map(|i| (i.label.as_str(), i.kind)).collect();
    assert_eq!(
        l,
        [
            ("n", Kind::Var),
            ("t", Kind::Var),
            ("Pont", Kind::Type),
            ("f", Kind::Func)
        ]
    );
}

#[test]
fn highlight_does_not_panic_on_odd_input() {
    let p = palette(true);
    let f = FontId::monospace(14.0);
    for t in [
        "",
        "\"unterminated",
        "/* open",
        "1..n",
        "a->b ≤ ∀",
        "Be:\nn∈N\n",
        "é ő ű ∃",
    ] {
        let j = highlight_spec(t, f.clone(), &p, &[1]);
        assert_eq!(j.text, t);
        let j = highlight_data(t, f.clone(), &p, &[1]);
        assert_eq!(j.text, t);
    }
}

fn suggestions(before: &str, typed: &str, lang: Lang) -> Vec<String> {
    let symbols =
        scan_symbols("Be: szamok∈N[1..n], n∈N, Pont=(x:Z x y:Z)\nFv: kisebb:N->L, kisebb(i)=i<3");
    let text = format!("{before}{typed}");
    let mut comp = Completion::default();
    let id = Id::unique("suggestions");
    comp.update(id, &text, text.chars().count(), true, lang, &symbols);
    comp.items.into_iter().map(|i| i.label).collect()
}

#[test]
fn suggestions_depend_on_the_context() {
    assert_eq!(suggestions("Be: p∈", "P", Lang::Spec), ["Pont"]);
    assert_eq!(suggestions("Be: p ∈ ", "n", Lang::Spec), ["N"]);
    assert_eq!(suggestions("Be: n∈N\n", "K", Lang::Spec)[0], "Ki:");
    assert_eq!(
        suggestions("Uf: x=", "s", Lang::Spec),
        ["sqrt", "szamok", "SUM", "SELECT", "SEARCH"]
    );
    assert_eq!(
        suggestions("Uf: x=", "S", Lang::Spec),
        ["SUM", "SELECT", "SEARCH", "sqrt", "szamok"]
    );
    assert!(suggestions("Uf: x=", "szamok", Lang::Spec).is_empty());
    assert!(suggestions("Uf: x=2", "s", Lang::Spec).is_empty());
    assert_eq!(suggestions("n: 1\n", "sz", Lang::Data), ["szamok"]);
    assert_eq!(suggestions("ok: ", "t", Lang::Data), ["true"]);
    assert!(suggestions("# ", "sz", Lang::Data).is_empty());
    assert!(suggestions("Uf: ", "s", Lang::Spec).len() <= 12);
}

#[test]
fn suggestions_follow_the_cursor() {
    let symbols = scan_symbols("Be: szamok∈N[1..n]");
    let id = Id::unique("follow");
    let mut comp = Completion::default();
    comp.update(id, "Uf: sz", 6, true, Lang::Spec, &symbols);
    assert!(comp.is_open_for(id) && comp.start == 4);
    assert!(!comp.is_open_for(Id::unique("another editor")));
    comp.update(id, "Uf: sz", 5, false, Lang::Spec, &symbols);
    assert!(comp.is_open_for(id));
    comp.update(id, "Uf: sz", 3, false, Lang::Spec, &symbols);
    assert!(!comp.is_open_for(id));
    comp.update(id, "Uf: sz", 6, false, Lang::Spec, &symbols);
    assert!(!comp.is_open_for(id));
}

fn sections(job: &LayoutJob, keep: impl Fn(&TextFormat) -> bool) -> Vec<&str> {
    job.sections
        .iter()
        .filter(|s| keep(&s.format))
        .map(|s| job.text[s.byte_range.start.0..s.byte_range.end.0].trim_end_matches([' ', '\t']))
        .collect()
}

#[test]
fn text_literals_are_highlighted_like_the_parser_reads_them() {
    let p = palette(false);
    let f = FontId::monospace(14.0);
    for (src, literal) in [
        ("s=\"a//b\" x", "\"a//b\""),
        ("s='/*' x", "'/*'"),
        ("s=„szó” x", "„szó”"),
        ("s=\"abc” x", "\"abc”"),
        ("s=\"open\nUf:", "\"open"),
    ] {
        let job = highlight_spec(src, f.clone(), &p, &[]);
        assert_eq!(job.text, src);
        assert_eq!(
            sections(&job, |f| f.color == p.string).concat(),
            literal,
            "{src}"
        );
    }
    let job = highlight_spec("[1..n]", f.clone(), &p, &[]);
    assert_eq!(sections(&job, |f| f.color == p.number), ["1"]);
    let job = highlight_data("n: 1\nx: [\ny: 2", f.clone(), &p, &[2]);
    let tinted: Vec<&str> = job
        .sections
        .iter()
        .filter(|s| s.format.background == p.error_bg)
        .map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0])
        .collect();
    assert_eq!(tinted.concat().trim_matches('\n'), "x: [");
    let job = highlight_data("x: [10, 20, 30, 40]", f, &p, &[]);
    assert_eq!(job.text, "x: [10, 20, 30, 40]");
    assert!(job.sections.len() <= 11, "{} runs", job.sections.len());
}
