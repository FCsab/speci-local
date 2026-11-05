#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")] // no console window on Windows

use eframe::egui::{
    self, Color32, Id, RichText, TextEdit, TextStyle, text::CCursor, text::CCursorRange,
};
use serde::{Deserialize, Serialize};
use speci_local::{Outcome, diag::Diagnostic, parse, run_test, scaffold};
use std::path::PathBuf;

// Nerd Font icons (0xProto Nerd Font)
const I_NEW: &str = "\u{f016}";
const I_OPEN: &str = "\u{f07c}";
const I_SAVE: &str = "\u{f0c7}";
const I_RUN: &str = "\u{f04b}";
const I_OK: &str = "\u{f00c}";
const I_FAIL: &str = "\u{f00d}";
const I_ADD: &str = "\u{f067}";
const I_DEL: &str = "\u{f068}";
const I_WAND: &str = "\u{f0d0}";

const GREEN: Color32 = Color32::from_rgb(46, 160, 67);
const RED: Color32 = Color32::from_rgb(218, 54, 51);

#[derive(Serialize, Deserialize, Clone)]
struct Doc {
    title: String,
    spec: String,
    tests: Vec<String>,
}

impl Default for Doc {
    fn default() -> Self {
        Doc {
            title: String::new(),
            spec: "Be:\nn∈N,\nszamok∈N[1..n]\n\nKi:\ndb∈N\n\nEf:\n-\n\nUf:\ndb=DARAB(i=1..n, szamok[i] mod 2 = 1)\n".into(),
            tests: vec!["n: 5\nszamok: [3, 72, 81, 4, 5]\ndb: 3\n".into()],
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct App {
    doc: Doc,
    current: usize,
    font: f32,
    show_s: bool,
    show_d: bool,
    show_v: bool,
    path: Option<PathBuf>,
    #[serde(skip)]
    results: Vec<Option<Outcome>>,
    #[serde(skip)]
    spec_diags: Vec<Diagnostic>,
    #[serde(skip)]
    status: String,
}

impl Default for App {
    fn default() -> Self {
        App {
            doc: Doc::default(),
            current: 0,
            font: 20.0,
            show_s: true,
            show_d: true,
            show_v: true,
            path: None,
            results: vec![],
            spec_diags: vec![],
            status: String::new(),
        }
    }
}

fn fonts(ctx: &egui::Context) {
    let mut f = egui::FontDefinitions::default();
    let add = |f: &mut egui::FontDefinitions, name: &str, bytes: &'static [u8]| {
        f.font_data.insert(
            name.into(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    };
    add(
        &mut f,
        "proto",
        include_bytes!("../assets/0xProtoNerdFontPropo-Regular.ttf"),
    );
    add(
        &mut f,
        "proto-mono",
        include_bytes!("../assets/0xProtoNerdFontMono-Regular.ttf"),
    );
    add(
        &mut f,
        "math",
        include_bytes!("../assets/NotoSansMath-Regular.ttf"),
    );
    // 0xProto lacks ∈ ∀ ∃ →, so Noto Sans Math comes right after it
    for (fam, main) in [
        (egui::FontFamily::Proportional, "proto"),
        (egui::FontFamily::Monospace, "proto-mono"),
    ] {
        let list = f.families.entry(fam).or_default();
        list.insert(0, main.into());
        list.insert(1, "math".into());
    }
    ctx.set_fonts(f);
}

/// Insert `s` at the editor's cursor (replacing any selection) and keep focus there.
fn insert_at_cursor(ctx: &egui::Context, id: Id, text: &mut String, s: &str) {
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let n = text.chars().count();
    let (a, b) = state
        .cursor
        .char_range()
        .map(|r| {
            let r = r.as_sorted_char_range();
            (r.start.0.min(n), r.end.0.min(n))
        })
        .unwrap_or((n, n));
    let byte = |c: usize| {
        text.char_indices()
            .nth(c)
            .map(|(i, _)| i)
            .unwrap_or(text.len())
    };
    let (ba, bb) = (byte(a), byte(b));
    text.replace_range(ba..bb, s);
    let mut cur = CCursor::default();
    cur.index.0 = a + s.chars().count();
    state.cursor.set_char_range(Some(CCursorRange::one(cur)));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

impl App {
    fn evaluate(&mut self) {
        self.results = vec![None; self.doc.tests.len()];
        match parse(&self.doc.spec) {
            Err(d) => {
                self.spec_diags = d;
                self.status = "The specification has errors".into();
            }
            Ok(spec) => {
                self.spec_diags.clear();
                self.results = self
                    .doc
                    .tests
                    .iter()
                    .map(|t| Some(run_test(&spec, t)))
                    .collect();
                let ok = self.results.iter().flatten().filter(|o| o.pass).count();
                self.status = format!("{ok} of {} test case(s) passed", self.doc.tests.len());
            }
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.path, as_new) {
            (Some(p), false) => Some(p.clone()),
            _ => {
                let name = if self.doc.title.trim().is_empty() {
                    "specification".to_string()
                } else {
                    self.doc.title.trim().to_string()
                };
                rfd::FileDialog::new()
                    .add_filter("Specification", &["json"])
                    .set_file_name(format!("{name}.spec.json"))
                    .save_file()
            }
        };
        let Some(path) = path else { return };
        let json = serde_json::to_string_pretty(&self.doc).expect("Doc always serializes");
        match std::fs::write(&path, json) {
            Ok(()) => {
                self.status = format!("Saved to {}", path.display());
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not save {}: {e}", path.display()),
        }
    }

    fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Specification", &["json"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Doc>(&s).map_err(|e| e.to_string()))
        {
            Ok(mut doc) => {
                if doc.tests.is_empty() {
                    doc.tests.push(String::new());
                }
                self.doc = doc;
                self.current = 0;
                self.results.clear();
                self.spec_diags.clear();
                self.status = format!("Opened {}", path.display());
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Specification editor");
            ui.add(
                TextEdit::singleline(&mut self.doc.title)
                    .hint_text("Enter the task name...")
                    .desired_width(260.0),
            );
            ui.add(egui::Slider::new(&mut self.font, 12.0..=28.0).suffix("px"));
            ui.toggle_value(&mut self.show_s, "S")
                .on_hover_text("Specification pane");
            ui.toggle_value(&mut self.show_d, "D")
                .on_hover_text("Data pane (test cases)");
            ui.toggle_value(&mut self.show_v, "V")
                .on_hover_text("Evaluation pane");
            ui.separator();
            if ui.button(format!("{I_NEW} New")).clicked() {
                let sure = rfd::MessageDialog::new()
                    .set_title("New specification")
                    .set_description("Discard the current specification and its test cases?")
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show();
                if sure == rfd::MessageDialogResult::Yes {
                    let font = self.font;
                    *self = App {
                        font,
                        ..App::default()
                    };
                    self.doc.spec = "Be:\n\nKi:\n\nEf:\n-\n\nUf:\n".into();
                    self.doc.tests = vec![String::new()];
                }
            }
            if ui.button(format!("{I_OPEN} Open")).clicked() {
                self.open();
            }
            if ui
                .button(format!("{I_SAVE} Save"))
                .on_hover_text("Ctrl+S")
                .clicked()
            {
                self.save(false);
            }
            if ui.button("Save as…").clicked() {
                self.save(true);
            }
        });
    }

    fn spec_pane(&mut self, ui: &mut egui::Ui) {
        let id = Id::unique("spec-editor");
        ui.horizontal(|ui| {
            ui.strong("Specification");
            ui.separator();
            ui.label("Insert:");
            for sym in ["∈", "∀", "∃", "≠", "≤", "≥"] {
                if ui.button(sym).clicked() {
                    insert_at_cursor(ui.ctx(), id, &mut self.doc.spec, sym);
                }
            }
        });
        egui::ScrollArea::both()
            .id_salt("spec-scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.doc.spec)
                        .id(id)
                        .code_editor()
                        .font(TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(24),
                );
            });
    }

    fn data_pane(&mut self, ui: &mut egui::Ui) {
        self.current = self.current.min(self.doc.tests.len().saturating_sub(1));
        ui.horizontal_wrapped(|ui| {
            ui.strong("Test cases");
            if ui.button(I_ADD).on_hover_text("Add a test case").clicked() {
                self.doc.tests.push(String::new());
                self.current = self.doc.tests.len() - 1;
            }
            if ui.button(I_DEL).on_hover_text("Remove this test case").clicked() && self.doc.tests.len() > 1 {
                self.doc.tests.remove(self.current);
                if self.current < self.results.len() {
                    self.results.remove(self.current);
                }
                self.current = self.current.min(self.doc.tests.len() - 1);
            }
            for i in 0..self.doc.tests.len() {
                let mut b = egui::Button::new(RichText::new((i + 1).to_string()).strong()).selected(i == self.current);
                if let Some(Some(o)) = self.results.get(i) {
                    b = b.fill(if o.pass { GREEN } else { RED });
                }
                if ui.add(b).clicked() {
                    self.current = i;
                }
            }
            if ui.button(format!("{I_WAND} Scaffold")).on_hover_text("Insert `name: ` lines for every declared variable (Shift+Space, Tab in the ELTE editor)").clicked() {
                match parse(&self.doc.spec) {
                    Ok(spec) => {
                        let t = &mut self.doc.tests[self.current];
                        if !t.is_empty() && !t.ends_with('\n') {
                            t.push('\n');
                        }
                        t.push_str(&scaffold(&spec, t));
                    }
                    Err(d) => {
                        self.spec_diags = d;
                        self.status = "Fix the specification first, then scaffold".into();
                    }
                }
            }
        });
        let cur = self.current;
        egui::ScrollArea::both()
            .id_salt("data-scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.doc.tests[cur])
                        .id(Id::unique(("data-editor", cur)))
                        .code_editor()
                        .font(TextStyle::Monospace)
                        .hint_text("n: 3\nnums: [1, 2, 3]")
                        .desired_width(f32::INFINITY)
                        .desired_rows(16),
                );
            });
    }

    fn eval_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(format!("{I_RUN} Evaluate")).strong())
                .on_hover_text("Ctrl+Enter")
                .clicked()
            {
                self.evaluate();
            }
            ui.label(&self.status);
        });
        ui.separator();
        let mono = |s: String| RichText::new(s).monospace();
        egui::ScrollArea::vertical()
            .id_salt("eval-scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                for d in &self.spec_diags {
                    ui.label(mono(d.render(&self.doc.spec, "")).color(RED));
                    ui.add_space(8.0);
                }
                // current tab first, then the rest
                let order = std::iter::once(self.current)
                    .chain((0..self.results.len()).filter(|i| *i != self.current));
                for i in order {
                    let Some(Some(o)) = self.results.get(i) else {
                        continue;
                    };
                    let (icon, word, color) = if o.pass {
                        (I_OK, "passed", GREEN)
                    } else {
                        (I_FAIL, "failed", RED)
                    };
                    ui.label(
                        RichText::new(format!("{icon} Test case {}: {word}", i + 1))
                            .strong()
                            .color(color),
                    );
                    let data = self.doc.tests.get(i).map(String::as_str).unwrap_or("");
                    for d in &o.diags {
                        ui.label(mono(d.render(&self.doc.spec, data)));
                    }
                    for (name, val, given) in &o.values {
                        let src = if *given { "" } else { "  (computed)" };
                        ui.label(mono(format!("  {name} = {val}{src}")));
                    }
                    for n in &o.notes {
                        ui.label(RichText::new(format!("Note: {n}")).italics());
                    }
                    ui.add_space(10.0);
                }
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let font = self.font;
        ui.ctx().all_styles_mut(|s| {
            for (ts, f) in s.text_styles.iter_mut() {
                f.size = match ts {
                    TextStyle::Heading => font + 4.0,
                    TextStyle::Small => (font - 4.0).max(10.0),
                    _ => font,
                };
            }
        });
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)) {
            self.evaluate();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save(false);
        }
        if self.results.len() != self.doc.tests.len() {
            self.results.resize(self.doc.tests.len(), None);
        }

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        if self.show_v {
            egui::Panel::bottom("eval")
                .resizable(true)
                .default_size(260.0)
                .show(ui, |ui| self.eval_pane(ui));
        }
        if self.show_s && self.show_d {
            egui::Panel::right("data")
                .resizable(true)
                .default_size(480.0)
                .show(ui, |ui| self.data_pane(ui));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if self.show_s {
                self.spec_pane(ui);
            } else if self.show_d {
                self.data_pane(ui);
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_title("speci-local"),
        ..Default::default()
    };
    eframe::run_native(
        "speci-local",
        options,
        Box::new(|cc| {
            fonts(&cc.egui_ctx);
            let app: App = cc
                .storage
                .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
                .unwrap_or_default();
            Ok(Box::new(app))
        }),
    )
}
