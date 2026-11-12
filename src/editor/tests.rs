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
                    None,
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
    assert_eq!(labels, ["kisebb", "KIVÁLASZT", "KIVÁLOGAT"]);
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
        ["sqrt", "szamok", "SUM", "SELECT", "SEARCH", "SZUMMA"]
    );
    assert_eq!(
        suggestions("Uf: x=", "S", Lang::Spec),
        ["SUM", "SELECT", "SEARCH", "SZUMMA", "sqrt", "szamok"]
    );
    assert!(suggestions("Uf: x=", "szamok", Lang::Spec).is_empty());
    assert!(suggestions("Uf: x=2", "s", Lang::Spec).is_empty());
    assert_eq!(suggestions("n: 1\n", "sz", Lang::Data), ["szamok"]);
    assert_eq!(suggestions("ok: ", "t", Lang::Data), ["true"]);
    assert_eq!(suggestions("ok: ", "ig", Lang::Data), ["igaz"]);
    assert_eq!(suggestions("Uf: x=", "KIVÁLO", Lang::Spec), ["KIVÁLOGAT"]);
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
fn hungarian_macro_names_are_highlighted_like_english_ones() {
    let p = palette(false);
    let f = FontId::monospace(14.0);
    let src = "s=SUM(i=1..n, x[i]) és d=SZUMMA(i=1..n, x[i]) és c=MEGSZÁMOL(i=1..n, x[i]>0)";
    let job = highlight_spec(src, f, &p, &[]);
    assert_eq!(
        sections(&job, |f| f.color == p.macro_),
        ["SUM", "SZUMMA", "MEGSZÁMOL"]
    );
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

mod vim_keys {
    use super::super::vim::{Mode, run_keys};

    /// (text, cursor, mode) after `keys` from char `p`; `⎋` is Escape.
    fn k(text: &str, p: usize, keys: &str) -> (String, usize, Mode) {
        run_keys(text, p, keys)
    }

    #[test]
    fn motions() {
        let t = "foo bar.baz\n  qux";
        assert_eq!(k(t, 0, "w").1, 4);
        assert_eq!(k(t, 0, "ww").1, 7);
        assert_eq!(k(t, 0, "W").1, 4);
        assert_eq!(k(t, 0, "e").1, 2);
        assert_eq!(k(t, 0, "$").1, 10);
        assert_eq!(k(t, 5, "0").1, 0);
        assert_eq!(k(t, 5, "j").1, 16); // clamped to the end of "  qux"
        assert_eq!(k(t, 0, "j^").1, 14);
        assert_eq!(k(t, 0, "G").1, 14);
        assert_eq!(k(t, 14, "gg").1, 0);
        assert_eq!(k(t, 0, "fb").1, 4);
        assert_eq!(k(t, 0, "fa;").1, 9);
        assert_eq!(k(t, 0, "tz").1, 9);
        assert_eq!(k(t, 7, "b").1, 4);
        assert_eq!(k("f(a, (b))", 1, "%").1, 8);
    }

    #[test]
    fn operators() {
        assert_eq!(k("foo bar baz", 0, "dw").0, "bar baz");
        assert_eq!(k("foo bar baz", 0, "d2w").0, "baz");
        assert_eq!(k("foo bar baz", 0, "2dw").0, "baz");
        assert_eq!(k("foo bar\nnext", 4, "dw").0, "foo \nnext");
        assert_eq!(k("foo bar baz", 0, "cwxy⎋").0, "xy bar baz");
        assert_eq!(k("a\nb\nc", 2, "dd"), ("a\nc".into(), 2, Mode::Normal));
        assert_eq!(k("a\nb\nc", 4, "dd").0, "a\nb");
        assert_eq!(k("a\nb\nc", 0, "2dd").0, "c");
        assert_eq!(k("foo bar", 4, "D").0, "foo ");
        assert_eq!(k("x = f(a, b)", 6, "ci(z⎋").0, "x = f(z)");
        assert_eq!(k("say \"hi there\"", 6, "di\"").0, "say \"\"");
        assert_eq!(k("foo bar baz", 5, "daw").0, "foo baz");
        assert_eq!(k("  ab\ncd", 3, "cc").0, "  \ncd");
        assert_eq!(k("ab\ncd", 0, ">j").0, "  ab\n  cd");
        assert_eq!(k("  ab", 0, "<<").0, "ab");
        assert_eq!(k("abc", 0, "x"), ("bc".into(), 0, Mode::Normal));
        assert_eq!(k("abc", 0, "~~").0, "ABc");
        assert_eq!(k("abc", 1, "rz").0, "azc");
        assert_eq!(k("a\n  b", 0, "J").0, "a b");
    }

    #[test]
    fn yank_put_undo_repeat() {
        assert_eq!(k("a\nb", 0, "yyp").0, "a\na\nb");
        assert_eq!(k("a\nb", 2, "yyP").0, "a\nb\nb");
        assert_eq!(k("a\nb", 2, "yyp").0, "a\nb\nb");
        assert_eq!(k("ab", 0, "xp").0, "ba");
        assert_eq!(k("foo bar", 0, "ywP").0, "foo foo bar");
        assert_eq!(k("abc", 0, "xxu").0, "bc");
        assert_eq!(k("abc", 0, "xxuu").0, "abc");
        assert_eq!(k("abc", 0, "xxuu\x12").0, "bc");
        assert_eq!(k("a b c d", 0, "dw..").0, "d");
        assert_eq!(k("x\ny", 0, "Ahi⎋j.").0, "xhi\nyhi");
        assert_eq!(k("abc", 0, "ihi⎋u").0, "abc");
    }

    #[test]
    fn insert_commands() {
        assert_eq!(k("ab", 0, "ix⎋"), ("xab".into(), 0, Mode::Normal));
        assert_eq!(k("ab", 0, "ax⎋").0, "axb");
        assert_eq!(k("ab", 1, "Ix⎋").0, "xab");
        assert_eq!(k("ab", 0, "Ax⎋").0, "abx");
        assert_eq!(k("  ab", 2, "ox⎋").0, "  ab\n  x");
        assert_eq!(k("ab", 0, "Ox⎋").0, "x\nab");
        assert_eq!(k("ab", 0, "i").2, Mode::Insert);
    }

    #[test]
    fn visual() {
        assert_eq!(k("foo bar", 0, "vlld").0, " bar");
        assert_eq!(k("foo bar", 0, "veyP").0, "foofoo bar");
        assert_eq!(k("a\nb\nc", 0, "Vjd").0, "c");
        assert_eq!(k("foo bar", 4, "viwcx⎋").0, "foo x");
        assert_eq!(k("abc", 0, "vlU").0, "ABc");
        assert_eq!(k("abc", 0, "v⎋").2, Mode::Normal);
    }
}

#[test]
fn vim_keys_through_the_widget() {
    struct V {
        text: String,
        comp: Completion,
        vim: Vim,
        first: bool,
    }
    let id = Id::unique("t-vim-editor");
    let mut h = Harness::builder()
        .with_context_setup(crate::app::fonts)
        .build_ui_state(
            move |ui, s: &mut V| {
                if s.first {
                    s.first = false;
                    set_cursor(ui.ctx(), id, 0);
                }
                code_editor(
                    ui,
                    id,
                    &mut s.text,
                    Lang::Spec,
                    &[],
                    &[],
                    &mut s.comp,
                    "",
                    Some(&mut s.vim),
                );
            },
            V {
                text: "foo bar\nbaz".into(),
                comp: Completion::default(),
                vim: Vim::default(),
                first: true,
            },
        );
    h.run_steps(3);
    // Normal mode: typed letters are commands
    for t in ["d", "w"] {
        h.event(egui::Event::Text(t.into()));
        h.run_steps(2);
    }
    assert_eq!(h.state().text, "bar\nbaz");
    // Insert mode types, Escape goes back without leaving the editor
    for t in ["A", "!"] {
        h.event(egui::Event::Text(t.into()));
        h.run_steps(2);
    }
    assert_eq!(h.state().text, "bar!\nbaz");
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(!h.state().vim.inserting());
    assert!(h.ctx.memory(|m| m.has_focus(id)));
    for t in ["j", "x", "u"] {
        h.event(egui::Event::Text(t.into()));
        h.run_steps(2);
    }
    assert_eq!(h.state().text, "bar!\nbaz");
    h.event(egui::Event::Text("x".into()));
    h.run_steps(2);
    // `j` keeps the column (clamped to "baz"), so `x` takes the z
    assert_eq!(h.state().text, "bar!\nba");
}
