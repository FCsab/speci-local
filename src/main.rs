#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")] // no console window on Windows

mod editor;

use editor::{Completion, Item, Lang, code_editor, cursor_of, replace, scan_symbols, set_cursor};
use eframe::egui::{
    self, Color32, CornerRadius, Id, Margin, RichText, Stroke, TextEdit, TextStyle, Theme,
};
use serde::{Deserialize, Serialize};
use speci_local::diag::{Diagnostic, Pane};
use speci_local::eval::{StepKind, Trace};
use speci_local::file::{Doc, from_clipboard, from_sp, to_compact, to_sp};
use speci_local::{Outcome, parse, run_test_with, scaffold, trace_test};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

// Nerd Font icons (0xProto Nerd Font)
const I_NEW: &str = "\u{f016}";
const I_OPEN: &str = "\u{f07c}";
const I_SAVE: &str = "\u{f0c7}";
const I_RUN: &str = "\u{f04b}";
const I_OK: &str = "\u{f058}";
const I_FAIL: &str = "\u{f057}";
const I_ADD: &str = "\u{f067}";
const I_DEL: &str = "\u{f1f8}";
const I_WAND: &str = "\u{f0d0}";
const I_GEAR: &str = "\u{f013}";
const I_SPEC: &str = "\u{f121}";
const I_DATA: &str = "\u{f1c0}";
const I_VIEW: &str = "\u{f0ae}";
const I_WARN: &str = "\u{f071}";
const I_BULB: &str = "\u{f0eb}";
const I_COPY: &str = "\u{f0c5}";
const I_PASTE: &str = "\u{f0ea}";
const I_FIRST: &str = "\u{f049}";
const I_PREV: &str = "\u{f048}";
const I_NEXT: &str = "\u{f051}";
const I_LAST: &str = "\u{f050}";
const STEP_BG: Color32 = Color32::from_rgb(250, 204, 21);

const GREEN: Color32 = Color32::from_rgb(46, 160, 67);
const RED: Color32 = Color32::from_rgb(229, 72, 77);
const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);

const SPEC_ID: &str = "spec-editor";

#[derive(Clone)]
enum Replace {
    New,
    Open,
    Load(PathBuf),
    Import(Doc),
}

struct EvalResult {
    outcomes: Result<Vec<Outcome>, Vec<Diagnostic>>,
    /// Steps of the selected test case, for the stepper.
    trace: Option<Trace>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
enum View {
    #[default]
    Results,
    Stepper,
    StepList,
}

/// An evaluation running on a worker thread.
struct Job {
    doc: Doc,
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<EvalResult>,
    started: f64,
}

impl Drop for Job {
    fn drop(&mut self) {
        // a newer edit superseded this run: let the worker stop early
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn doc_hash(d: &Doc) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    (&d.spec, &d.tests).hash(&mut h);
    h.finish()
}

fn sample() -> Doc {
    Doc {
        title: "Páratlan számok".into(),
        spec: "Be:\nn∈N,\nszamok∈N[1..n]\n\nKi:\ndb∈N\n\nEf:\n-\n\nUf:\ndb=DARAB(i=1..n, szamok[i] mod 2 = 1)\n".into(),
        tests: vec!["n: 5\nszamok: [3, 72, 81, 4, 5]\ndb: 3\n".into()],
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct App {
    doc: Doc,
    /// What is on disk (or the initial state) — `doc != saved` means unsaved changes.
    saved: Doc,
    current: usize,
    font: f32,
    show_s: bool,
    show_d: bool,
    show_v: bool,
    path: Option<PathBuf>,
    /// Re-evaluate automatically shortly after every change.
    auto: bool,
    view: View,
    #[serde(skip)]
    trace: Option<Trace>,
    #[serde(skip)]
    step: usize,
    /// Test case the current trace belongs to.
    #[serde(skip)]
    traced_test: usize,
    /// Fallback when the system clipboard can't be read: the user pastes here.
    #[serde(skip)]
    paste_box: Option<String>,
    #[serde(skip)]
    results: Vec<Option<Outcome>>,
    #[serde(skip)]
    job: Option<Job>,
    #[serde(skip)]
    confirm: Option<Replace>,
    /// (hash of the document, time it last changed)
    #[serde(skip)]
    last_change: (u64, f64),
    /// hash of the document the last evaluation was started for
    #[serde(skip)]
    submitted: u64,
    #[serde(skip)]
    spec_diags: Vec<Diagnostic>,
    /// The document as it was when last evaluated (error highlights only apply to unchanged text).
    #[serde(skip)]
    evaluated: Option<Doc>,
    #[serde(skip)]
    status: String,
    #[serde(skip)]
    comp: Completion,
    #[serde(skip)]
    symbols: (String, Vec<Item>),
    #[serde(skip)]
    applied: (f32, String),
}

impl Default for App {
    fn default() -> Self {
        let doc = sample();
        App {
            saved: doc.clone(),
            doc,
            current: 0,
            font: 18.0,
            show_s: true,
            show_d: true,
            show_v: true,
            path: None,
            auto: true,
            view: View::Results,
            trace: None,
            step: 0,
            traced_test: 0,
            paste_box: None,
            results: vec![],
            job: None,
            confirm: None,
            last_change: (0, 0.0),
            submitted: 0,
            spec_diags: vec![],
            evaluated: None,
            status: String::new(),
            comp: Completion::default(),
            symbols: (String::new(), vec![]),
            applied: (0.0, String::new()),
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

fn theme(ctx: &egui::Context, font: f32) {
    ctx.all_styles_mut(|s| {
        for (ts, f) in s.text_styles.iter_mut() {
            f.size = match ts {
                TextStyle::Heading => font + 4.0,
                TextStyle::Small => (font * 0.72).max(10.0),
                TextStyle::Button => font * 0.9,
                _ => font,
            };
        }
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = font + 10.0;
        let r = CornerRadius::same(6);
        for w in [
            &mut s.visuals.widgets.noninteractive,
            &mut s.visuals.widgets.inactive,
            &mut s.visuals.widgets.hovered,
            &mut s.visuals.widgets.active,
            &mut s.visuals.widgets.open,
        ] {
            w.corner_radius = r;
        }
        s.visuals.window_corner_radius = CornerRadius::same(10);
        s.visuals.menu_corner_radius = CornerRadius::same(8);
        s.visuals.selection.bg_fill = ACCENT.linear_multiply(0.55);
        s.visuals.hyperlink_color = ACCENT;
    });
    ctx.style_mut_of(Theme::Dark, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_rgb(0x15, 0x17, 0x1c);
        v.window_fill = Color32::from_rgb(0x1d, 0x20, 0x27);
        v.extreme_bg_color = Color32::from_rgb(0x1b, 0x1e, 0x24);
        v.faint_bg_color = Color32::from_rgb(0x22, 0x26, 0x2e);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x26, 0x2a, 0x33);
        v.widgets.inactive.bg_fill = Color32::from_rgb(0x26, 0x2a, 0x33);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x30, 0x35, 0x40);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2e, 0x37));
    });
    ctx.style_mut_of(Theme::Light, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_rgb(0xf3, 0xf4, 0xf6);
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(0xea, 0xec, 0xf0);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0xe4, 0xe7, 0xeb);
        v.widgets.inactive.bg_fill = Color32::from_rgb(0xe4, 0xe7, 0xeb);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xd8, 0xdc, 0xe2);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xdc, 0xdf, 0xe4));
    });
}

/// Rounded "card" holding an editor or results.
fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(4))
}

fn pane_title(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.label(
        RichText::new(format!("{icon}  {title}"))
            .strong()
            .color(ui.visuals().strong_text_color()),
    );
}

/// char index of (line, col), both 1-based
fn char_index(text: &str, line: usize, col: usize) -> usize {
    let before: usize = text
        .split('\n')
        .take(line.saturating_sub(1))
        .map(|l| l.chars().count() + 1)
        .sum();
    before + col.saturating_sub(1)
}

fn line_col(text: &str, idx: usize) -> (usize, usize) {
    let before: Vec<char> = text.chars().take(idx).collect();
    let line = before.iter().filter(|c| **c == '\n').count() + 1;
    let col = before.iter().rev().take_while(|c| **c != '\n').count() + 1;
    (line, col)
}

impl App {
    fn dirty(&self) -> bool {
        self.doc != self.saved
    }

    /// Run an action that replaces the document, asking first if there are unsaved changes.
    fn replace_doc(&mut self, action: Replace) {
        if self.dirty() {
            self.confirm = Some(action);
            return;
        }
        match action {
            Replace::New => self.new_doc(),
            Replace::Open => {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Specification (*.sp)", &["sp"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.load(&p);
                }
            }
            Replace::Load(p) => self.load(&p),
            Replace::Import(doc) => {
                let font = self.font;
                *self = App {
                    font,
                    auto: self.auto,
                    view: self.view,
                    show_s: self.show_s,
                    show_d: self.show_d,
                    show_v: self.show_v,
                    ..App::default()
                };
                self.doc = doc;
                self.saved = Doc::default(); // imported, not saved anywhere yet
                self.status = "Project read from the clipboard (not saved to a file yet)".into();
            }
        }
    }

    /// The "unsaved changes" dialog (in-app, so it works without zenity/kdialog on Linux).
    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else {
            return;
        };
        let mut choice = None;
        let modal = egui::Modal::new(Id::unique("discard-dialog")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading("Unsaved changes");
            ui.label("The current specification has changes that are not saved yet.");
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(format!("{I_SAVE}  Save first")).clicked() {
                    choice = Some("save");
                }
                if ui.button("Discard changes").clicked() {
                    choice = Some("discard");
                }
                if ui.button("Cancel").clicked() {
                    choice = Some("cancel");
                }
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some("cancel");
        }
        match choice {
            Some("save") => {
                self.confirm = None;
                self.save(false);
                if !self.dirty() {
                    self.replace_doc(action);
                }
            }
            Some("discard") => {
                self.confirm = None;
                self.saved = self.doc.clone(); // nothing left to protect
                self.replace_doc(action);
            }
            Some(_) => self.confirm = None,
            None => {}
        }
    }

    /// Start evaluating the current document in the background (replacing any running evaluation).
    fn evaluate(&mut self, now: f64) {
        let doc = self.doc.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let (d, c) = (doc.clone(), cancel.clone());
        let test = self.current;
        self.traced_test = test;
        // big stack: deeply nested formulas and recursive functions are evaluated recursively
        let spawned = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let (outcomes, trace) = match parse(&d.spec) {
                    Err(diags) => (Err(diags), None),
                    Ok(spec) => {
                        let outcomes = d
                            .tests
                            .iter()
                            .map(|t| run_test_with(&spec, t, &c))
                            .collect();
                        let trace = d.tests.get(test).and_then(|t| trace_test(&spec, t, &c));
                        (Ok(outcomes), trace)
                    }
                };
                if !c.load(Ordering::Relaxed) {
                    let _ = tx.send(EvalResult { outcomes, trace });
                }
            });
        self.submitted = doc_hash(&doc);
        match spawned {
            Ok(_) => {
                self.job = Some(Job {
                    doc,
                    cancel,
                    rx,
                    started: now,
                })
            }
            Err(e) => self.status = format!("Could not start the evaluation: {e}"),
        }
    }

    /// Pick up a finished background evaluation.
    fn poll_job(&mut self) {
        let Some(job) = &self.job else { return };
        let r = match job.rx.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = "The evaluation crashed (this is a bug in speci-local)".into();
                self.job = None;
                return;
            }
        };
        let job = self.job.take().unwrap();
        self.trace = r.trace;
        match r.outcomes {
            Err(d) => {
                self.spec_diags = d;
                self.results = vec![None; job.doc.tests.len()];
                self.status = "The specification has errors".into();
            }
            Ok(outcomes) => {
                self.spec_diags.clear();
                let ok = outcomes.iter().filter(|o| o.pass).count();
                self.status = format!("{ok} of {} test case(s) passed", outcomes.len());
                self.results = outcomes.into_iter().map(Some).collect();
            }
        }
        self.evaluated = Some(job.doc.clone());
    }

    fn load(&mut self, path: &Path) {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut doc = from_sp(&text);
                if doc.title.is_empty() {
                    doc.title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                }
                self.saved = doc.clone();
                self.doc = doc;
                self.current = 0;
                self.results.clear();
                self.spec_diags.clear();
                self.evaluated = None;
                self.path = Some(path.to_path_buf());
                self.status = format!("Opened {}", path.display());
                self.submitted = 0; // evaluated on the next frame
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.path, as_new) {
            (Some(p), false) => Some(p.clone()),
            _ => {
                let name = if self.doc.title.trim().is_empty() {
                    "specification"
                } else {
                    self.doc.title.trim()
                };
                let name: String = name
                    .chars()
                    .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
                    .collect();
                rfd::FileDialog::new()
                    .add_filter("Specification (*.sp)", &["sp"])
                    .set_file_name(format!("{name}.sp"))
                    .save_file()
            }
        };
        let Some(mut path) = path else { return };
        if path.extension().is_none_or(|e| e != "sp") {
            path.set_extension("sp");
        }
        match std::fs::write(&path, to_sp(&self.doc)) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.saved = self.doc.clone();
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not save {}: {e}", path.display()),
        }
    }

    fn new_doc(&mut self) {
        let doc = Doc {
            title: String::new(),
            spec: "Be:\n\nKi:\n\nEf:\n-\n\nUf:\n".into(),
            tests: vec![String::new()],
        };
        *self = App {
            font: self.font,
            auto: self.auto,
            show_s: self.show_s,
            show_d: self.show_d,
            show_v: self.show_v,
            saved: doc.clone(),
            doc,
            ..App::default()
        };
    }

    fn jump(&mut self, ctx: &egui::Context, d: &Diagnostic, test: usize) {
        if d.line == 0 {
            return;
        }
        let (id, text) = match d.pane {
            Pane::Data => {
                self.current = test;
                (Id::unique(("data-editor", test)), &self.doc.tests[test])
            }
            _ => (Id::unique(SPEC_ID), &self.doc.spec),
        };
        set_cursor(ctx, id, char_index(text, d.line, d.col));
    }

    fn symbols(&mut self) -> Vec<Item> {
        if self.symbols.0 != self.doc.spec {
            self.symbols = (self.doc.spec.clone(), scan_symbols(&self.doc.spec));
        }
        self.symbols.1.clone()
    }

    /// Lines to tint red in the spec editor and in test tab `test`'s data editor.
    fn error_lines(&self, pane: Pane, test: usize) -> Vec<usize> {
        let Some(ev) = &self.evaluated else {
            return vec![];
        };
        let unchanged = match pane {
            Pane::Spec => ev.spec == self.doc.spec,
            _ => ev.tests.get(test) == self.doc.tests.get(test) && ev.spec == self.doc.spec,
        };
        if !unchanged {
            return vec![];
        }
        let mut diags: Vec<&Diagnostic> = self.spec_diags.iter().collect();
        diags.extend(
            self.results
                .get(test)
                .into_iter()
                .flatten()
                .flat_map(|o| &o.diags),
        );
        diags
            .iter()
            .filter(|d| d.pane == pane && d.line > 0)
            .map(|d| d.line)
            .collect()
    }

    // ---------------------------------------------------------------- panes

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("speci")
                    .strong()
                    .size(self.font + 2.0)
                    .color(ACCENT),
            );
            ui.label(RichText::new("local").size(self.font + 2.0).weak());
            ui.add_space(12.0);
            ui.add(
                TextEdit::singleline(&mut self.doc.title)
                    .hint_text("Untitled task")
                    .font(TextStyle::Heading)
                    .frame(egui::Frame::NONE)
                    .desired_width(320.0),
            );
            if self.dirty() {
                ui.label(RichText::new("●").color(ui.visuals().weak_text_color()))
                    .on_hover_text("Unsaved changes");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::widgets::global_theme_preference_buttons(ui);
                ui.menu_button(I_GEAR, |ui| {
                    ui.label("Font size");
                    ui.add(egui::Slider::new(&mut self.font, 12.0..=28.0).suffix(" px"));
                })
                .response
                .on_hover_text("Settings");
                ui.separator();
                ui.toggle_value(&mut self.show_v, I_VIEW)
                    .on_hover_text("Evaluation pane");
                ui.toggle_value(&mut self.show_d, I_DATA)
                    .on_hover_text("Test cases pane");
                ui.toggle_value(&mut self.show_s, I_SPEC)
                    .on_hover_text("Specification pane");
                ui.separator();
                if ui
                    .button(I_SAVE)
                    .on_hover_text("Save (Ctrl+S)\nShift-click: Save as… (Ctrl+Shift+S)")
                    .clicked()
                {
                    let shift = ui.input(|i| i.modifiers.shift);
                    self.save(shift);
                }
                if ui.button(I_PASTE).on_hover_text("Read a project from the clipboard (Ctrl+Shift+V)").clicked() {
                    self.paste_project();
                }
                if ui.button(I_COPY).on_hover_text("Copy the whole project to the clipboard as one compact line (Ctrl+Shift+C)").clicked() {
                    self.copy_project(ui.ctx());
                }
                ui.separator();
                if ui
                    .button(I_OPEN)
                    .on_hover_text("Open a .sp file (Ctrl+O)")
                    .clicked()
                {
                    self.replace_doc(Replace::Open);
                }
                if ui
                    .button(I_NEW)
                    .on_hover_text("New specification (Ctrl+N)")
                    .clicked()
                {
                    self.replace_doc(Replace::New);
                }
            });
        });
    }

    fn spec_pane(&mut self, ui: &mut egui::Ui) {
        let id = Id::unique(SPEC_ID);
        ui.horizontal(|ui| {
            pane_title(ui, I_SPEC, "Specification");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // right-to-left layout: iterate backwards so they read ∈ ∀ ∃ ≠ ≤ ≥
                for (n, (_, sym)) in editor::SYMBOL_KEYS.iter().enumerate().rev() {
                    let sym = *sym;
                    if ui
                        .button(RichText::new(sym).monospace())
                        .on_hover_text(format!("Insert {sym} (Alt+{})", n + 1))
                        .clicked()
                    {
                        let n = self.doc.spec.chars().count();
                        let (a, b) = cursor_of(ui.ctx(), id).unwrap_or((n, n));
                        replace(ui.ctx(), id, &mut self.doc.spec, a, b, sym);
                    }
                }
            });
        });
        let symbols = self.symbols();
        let errors = self.error_lines(Pane::Spec, self.current);
        card(ui).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("spec-scroll")
                .auto_shrink(false)
                .show(ui, |ui| {
                    code_editor(
                        ui,
                        id,
                        &mut self.doc.spec,
                        Lang::Spec,
                        &symbols,
                        &errors,
                        &mut self.comp,
                        "Be:\nn∈N\n\nKi:\n…",
                    );
                });
        });
    }

    fn data_pane(&mut self, ui: &mut egui::Ui) {
        self.current = self.current.min(self.doc.tests.len().saturating_sub(1));
        ui.horizontal(|ui| {
            pane_title(ui, I_DATA, "Test cases");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{I_WAND} Scaffold")).on_hover_text("Insert `name: ` lines for every declared variable\n(Shift+Space, Tab in the ELTE editor)").clicked() {
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
                            self.evaluated = Some(self.doc.clone());
                            self.status = "Fix the specification first, then scaffold".into();
                        }
                    }
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for i in 0..self.doc.tests.len() {
                let (fill, text) = match self.results.get(i) {
                    Some(Some(o)) if o.pass => (GREEN, Color32::WHITE),
                    Some(Some(_)) => (RED, Color32::WHITE),
                    _ => (
                        ui.visuals().widgets.inactive.weak_bg_fill,
                        ui.visuals().text_color(),
                    ),
                };
                let stroke = if i == self.current {
                    Stroke::new(2.0, ACCENT)
                } else {
                    Stroke::NONE
                };
                let b = egui::Button::new(RichText::new((i + 1).to_string()).strong().color(text))
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(12))
                    .min_size(egui::vec2(34.0, 0.0));
                if ui.add(b).clicked() {
                    self.current = i;
                }
            }
            if ui.button(I_ADD).on_hover_text("Add a test case").clicked() {
                self.doc.tests.push(String::new());
                self.current = self.doc.tests.len() - 1;
            }
            if self.doc.tests.len() > 1
                && ui
                    .button(I_DEL)
                    .on_hover_text("Delete this test case")
                    .clicked()
            {
                self.doc.tests.remove(self.current);
                if self.current < self.results.len() {
                    self.results.remove(self.current);
                }
                if let Some(ev) = &mut self.evaluated
                    && self.current < ev.tests.len()
                {
                    ev.tests.remove(self.current);
                }
                self.current = self.current.min(self.doc.tests.len() - 1);
            }
        });
        let cur = self.current;
        let symbols = self.symbols();
        let errors = self.error_lines(Pane::Data, cur);
        card(ui).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt(("data-scroll", cur))
                .auto_shrink(false)
                .show(ui, |ui| {
                    code_editor(
                        ui,
                        Id::unique(("data-editor", cur)),
                        &mut self.doc.tests[cur],
                        Lang::Data,
                        &symbols,
                        &errors,
                        &mut self.comp,
                        "n: 3\nnums: [1, 2, 3]",
                    );
                });
        });
    }

    fn eval_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            pane_title(ui, I_VIEW, "Evaluation");
            ui.selectable_value(&mut self.view, View::Results, "Results");
            ui.selectable_value(&mut self.view, View::Stepper, "Stepper")
                .on_hover_text("Step through how the selected test case is evaluated (← / →)");
            ui.selectable_value(&mut self.view, View::StepList, "Step list");
            ui.separator();
            let run = egui::Button::new(
                RichText::new(format!("{I_RUN}  Evaluate"))
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(ACCENT);
            if ui.add(run).on_hover_text("Ctrl+Enter").clicked() {
                self.evaluate(ui.input(|i| i.time));
            }
            ui.checkbox(&mut self.auto, "Auto")
                .on_hover_text("Re-evaluate automatically shortly after every change");
            // only show the spinner for runs that take noticeably long
            if self
                .job
                .as_ref()
                .is_some_and(|j| ui.input(|i| i.time) - j.started > 0.25)
            {
                ui.add(egui::Spinner::new());
                ui.label(RichText::new("Evaluating…").weak());
            }
            let ok = self.results.iter().flatten().filter(|o| o.pass).count();
            let bad = self.results.iter().flatten().filter(|o| !o.pass).count();
            if ok > 0 {
                ui.label(RichText::new(format!("{I_OK} {ok} passed")).color(GREEN));
            }
            if bad > 0 {
                ui.label(RichText::new(format!("{I_FAIL} {bad} failed")).color(RED));
            }
            if !self.spec_diags.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{I_WARN} {} problem(s) in the specification",
                        self.spec_diags.len()
                    ))
                    .color(RED),
                );
            }
        });
        if self.view != View::Results {
            card(ui).inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.steps_ui(ui);
            });
            return;
        }
        let mut jump: Option<(Diagnostic, usize)> = None;
        let mut select = None;
        card(ui).inner_margin(Margin::same(10)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("eval-scroll").auto_shrink(false).show(ui, |ui| {
                if self.spec_diags.is_empty() && self.results.iter().all(Option::is_none) {
                    let msg = if self.auto { "Results appear here as you type." } else { "Press Evaluate (Ctrl+Enter) to run every test case against the specification." };
                    ui.label(RichText::new(msg).weak());
                }
                for d in &self.spec_diags {
                    if diag_ui(ui, d, &self.doc.spec, "") {
                        jump = Some((d.clone(), self.current));
                    }
                }
                // failures first; within each group the selected tab leads
                let mut order: Vec<usize> = (0..self.results.len()).collect();
                order.sort_by_key(|i| (self.results[*i].as_ref().is_some_and(|o| o.pass), *i != self.current, *i));
                for i in order {
                    let Some(Some(o)) = self.results.get(i) else { continue };
                    let (icon, word, color) = if o.pass { (I_OK, "passed", GREEN) } else { (I_FAIL, "failed", RED) };
                    let head = ui.add(egui::Label::new(RichText::new(format!("{icon}  Test case {}  ·  {word}", i + 1)).strong().color(color)).sense(egui::Sense::click()));
                    if head.on_hover_text("Show this test case").clicked() {
                        select = Some(i);
                    }
                    let data = self.doc.tests.get(i).map(String::as_str).unwrap_or("");
                    ui.indent(("res", i), |ui| {
                        for d in &o.diags {
                            if diag_ui(ui, d, &self.doc.spec, data) {
                                jump = Some((d.clone(), i));
                            }
                        }
                        if !o.values.is_empty() {
                            egui::Grid::new(("vals", i)).num_columns(2).spacing([16.0, 2.0]).show(ui, |ui| {
                                for (name, val, given) in &o.values {
                                    ui.label(RichText::new(name).monospace().strong());
                                    let mut t = RichText::new(format!("= {val}")).monospace();
                                    if !*given {
                                        t = t.italics();
                                    }
                                    let l = ui.label(t);
                                    if !*given {
                                        l.on_hover_text("Computed by the specification (not given in the test case)");
                                    }
                                    ui.end_row();
                                }
                            });
                        }
                        for n in &o.notes {
                            ui.label(RichText::new(format!("{I_BULB} {n}")).weak());
                        }
                    });
                    ui.add_space(10.0);
                }
            });
        });
        if let Some(i) = select {
            self.current = i;
        }
        if let Some((d, i)) = jump {
            self.jump(ui.ctx(), &d, i);
        }
    }

    /// Stepper and step list (the original editor's visualization pane).
    fn steps_ui(&mut self, ui: &mut egui::Ui) {
        let Some(trace) = &self.trace else {
            let msg = if self.spec_diags.is_empty() {
                "No steps yet. They appear once the selected test case's data is valid."
            } else {
                "Fix the specification first; the steps appear once it parses."
            };
            ui.label(RichText::new(msg).weak());
            return;
        };
        let src = self
            .evaluated
            .as_ref()
            .map(|d| d.spec.as_str())
            .unwrap_or("");
        let n = trace.steps.len();
        if n == 0 {
            ui.label(
                RichText::new("Nothing to step through: the conditions contain only constants.")
                    .weak(),
            );
            return;
        }
        self.step = self.step.min(n - 1);
        let snippet = |span: (usize, usize)| -> String {
            let t = src
                .get(span.0..span.1)
                .unwrap_or("?")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if t.chars().count() > 70 {
                format!("{}…", t.chars().take(69).collect::<String>())
            } else {
                t
            }
        };
        let describe = |st: &speci_local::eval::Step| match &st.kind {
            StepKind::Value(v) => format!("{}  =  {v}", snippet(st.span)),
            StepKind::Error(t) => format!("{}  ✗  {t}", snippet(st.span)),
            StepKind::Enter(c) => format!("↓ {c}"),
            StepKind::Leave(r) => format!("↑ {r}"),
        };
        let mut step = self.step;
        let note = format!(
            "Test case {}{}",
            self.traced_test + 1,
            if trace.truncated {
                format!(" · only the first {n} steps are shown")
            } else {
                String::new()
            }
        );

        if self.view == View::StepList {
            ui.label(RichText::new(note).weak());
            let row_h = ui.text_style_height(&TextStyle::Monospace) + 6.0;
            egui::ScrollArea::vertical()
                .id_salt("step-list")
                .auto_shrink(false)
                .show_rows(ui, row_h, n, |ui, range| {
                    for i in range {
                        let st = &trace.steps[i];
                        let text = format!(
                            "{:>5}  {}{:<5} {}",
                            i + 1,
                            "  ".repeat(st.depth),
                            st.label,
                            describe(st)
                        );
                        let mut rt = RichText::new(text).monospace();
                        if matches!(st.kind, StepKind::Error(_)) {
                            rt = rt.color(RED);
                        }
                        if ui
                            .selectable_label(i == step, rt)
                            .on_hover_text("Show this step in the stepper")
                            .clicked()
                        {
                            step = i;
                            self.view = View::Stepper;
                        }
                    }
                });
            self.step = step;
            return;
        }

        // stepper: ← / → while no text field has the keyboard
        if ui.memory(|m| m.focused().is_none()) {
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                    step = (step + 1).min(n - 1);
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                    step = step.saturating_sub(1);
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.button(I_FIRST).on_hover_text("First step").clicked() {
                step = 0;
            }
            if ui
                .button(I_PREV)
                .on_hover_text("Previous step (←)")
                .clicked()
            {
                step = step.saturating_sub(1);
            }
            if ui.button(I_NEXT).on_hover_text("Next step (→)").clicked() {
                step = (step + 1).min(n - 1);
            }
            if ui.button(I_LAST).on_hover_text("Last step").clicked() {
                step = n - 1;
            }
            ui.spacing_mut().slider_width = (ui.available_width() - 260.0).clamp(120.0, 600.0);
            let mut one_based = step + 1;
            ui.add(egui::Slider::new(&mut one_based, 1..=n).text(format!("of {n}")));
            step = one_based - 1;
            ui.label(RichText::new(note).weak());
        });
        let st = &trace.steps[step];
        ui.add_space(6.0);
        // the whole formula, with the part evaluated in this step highlighted
        let line_start = src
            .get(..st.root.0)
            .and_then(|b| b.rfind('\n'))
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_end = src
            .get(st.root.1..)
            .and_then(|a| a.find('\n'))
            .map(|i| i + st.root.1)
            .unwrap_or(src.len());
        let font = TextStyle::Monospace.resolve(ui.style());
        let normal = ui.visuals().text_color();
        let mut job = egui::text::LayoutJob::default();
        for (a, b, hl) in [
            (line_start, st.span.0, false),
            (st.span.0, st.span.1, true),
            (st.span.1, line_end, false),
        ] {
            if let Some(t) = src.get(a..b) {
                let mut f = egui::TextFormat::simple(
                    font.clone(),
                    if hl { Color32::BLACK } else { normal },
                );
                if hl {
                    f.background = STEP_BG;
                }
                job.append(t, 0.0, f);
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(&st.label).strong().color(ACCENT));
            if st.depth > 0 {
                ui.label(
                    RichText::new(format!("inside a function call (depth {})", st.depth)).weak(),
                );
            }
        });
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::horizontal()
                    .id_salt("step-formula")
                    .show(ui, |ui| ui.label(job));
            });
        ui.add_space(4.0);
        let color = match &st.kind {
            StepKind::Error(_) => RED,
            StepKind::Enter(_) | StepKind::Leave(_) => ACCENT,
            StepKind::Value(_) => ui.visuals().strong_text_color(),
        };
        ui.label(RichText::new(describe(st)).monospace().color(color));
        if !st.scope.is_empty() {
            let vars: Vec<String> = st.scope.iter().map(|(n, v)| format!("{n} = {v}")).collect();
            ui.label(RichText::new(vars.join("   ")).monospace().weak());
        }
        self.step = step;
    }

    fn copy_project(&mut self, ctx: &egui::Context) {
        let text = to_compact(&self.doc);
        self.status = format!(
            "Copied the whole project to the clipboard ({} characters)",
            text.len()
        );
        ctx.copy_text(text);
    }

    fn paste_project(&mut self) {
        match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(text) => self.import_text(&text),
            // e.g. no clipboard access on this desktop: let the user paste into a text box
            Err(_) => self.paste_box = Some(String::new()),
        }
    }

    fn import_text(&mut self, text: &str) {
        match from_clipboard(text) {
            Ok(doc) => self.replace_doc(Replace::Import(doc)),
            Err(e) => self.status = e,
        }
    }

    fn paste_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.paste_box.take() else {
            return;
        };
        let mut done = false;
        let modal = egui::Modal::new(Id::unique("paste-dialog")).show(ctx, |ui| {
            ui.set_width(520.0);
            ui.heading("Paste a project");
            ui.label("The clipboard couldn't be read directly. Paste the copied project (SPECI1:…) or a specification here with Ctrl+V:");
            ui.add(TextEdit::multiline(&mut text).desired_rows(6).desired_width(f32::INFINITY).font(TextStyle::Monospace));
            ui.horizontal(|ui| {
                if ui.button(format!("{I_PASTE}  Import")).clicked() {
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    text.clear();
                    done = true;
                }
            });
        });
        if modal.should_close() && !done {
            return; // closed with Escape or a click outside
        }
        if done {
            if !text.trim().is_empty() {
                self.import_text(&text);
            }
        } else {
            self.paste_box = Some(text);
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let spec_id = Id::unique(SPEC_ID);
        let data_id = Id::unique(("data-editor", self.current));
        let pos = [
            (spec_id, &self.doc.spec),
            (data_id, &self.doc.tests[self.current]),
        ]
        .into_iter()
        .find(|(id, _)| ui.memory(|m| m.has_focus(*id)))
        .and_then(|(id, t)| cursor_of(ui.ctx(), id).map(|c| line_col(t, c.1)));
        let file = match &self.path {
            Some(p) => p.display().to_string(),
            None => "not saved yet".into(),
        };
        ui.horizontal(|ui| {
            // right side first so the (possibly long) path can be truncated to the remaining width
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );
                if let Some((l, c)) = pos {
                    ui.label(RichText::new(format!("Ln {l}, Col {c}")).small().weak());
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if self.dirty() {
                        ui.label(RichText::new("● modified").small().color(ACCENT));
                    }
                    if !self.status.is_empty() {
                        ui.add(egui::Label::new(RichText::new(&self.status).small()).truncate());
                    }
                    ui.add(egui::Label::new(RichText::new(file).small().weak()).truncate())
                        .on_hover_text("File of the current specification");
                });
            });
        });
    }
}

/// One diagnostic as a card. Returns true if the user clicked "go to".
fn diag_ui(ui: &mut egui::Ui, d: &Diagnostic, spec: &str, data: &str) -> bool {
    let mut go = false;
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(RED.linear_multiply(if dark { 0.10 } else { 0.07 }))
        .stroke(Stroke::new(1.0, RED.linear_multiply(0.45)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(I_WARN).color(RED));
                ui.label(RichText::new(&d.title).strong());
                if d.line > 0 {
                    let where_ = if d.pane == Pane::Data {
                        "test data"
                    } else {
                        "spec"
                    };
                    if ui
                        .link(format!("{where_} {}:{}", d.line, d.col))
                        .on_hover_text("Go to this position")
                        .clicked()
                    {
                        go = true;
                    }
                }
            });
            let src = if d.pane == Pane::Data { data } else { spec };
            if let Some(line) = src.lines().nth(d.line.wrapping_sub(1)) {
                let num = d.line.to_string();
                let code = format!(
                    "{num} │ {line}\n{} │ {}^",
                    " ".repeat(num.len()),
                    " ".repeat(d.col.saturating_sub(1))
                );
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new(code).monospace());
                    });
            }
            if !d.explanation.is_empty() {
                ui.label(&d.explanation);
            }
            if let Some(h) = &d.hint {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{I_BULB} Fix:"))
                            .strong()
                            .color(ACCENT),
                    );
                    ui.label(h);
                });
            }
        });
    ui.add_space(4.0);
    go
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.applied.0 != self.font {
            theme(&ctx, self.font);
            self.applied.0 = self.font;
        }
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.sp".into());
        let title = format!(
            "{}{name} — speci-local",
            if self.dirty() { "● " } else { "" }
        );
        if self.applied.1 != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.applied.1 = title;
        }

        // shortcuts (Ctrl+Shift+S must be checked before Ctrl+S)
        let (run, save_as, save, open, new) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter),
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::S,
                ),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::N),
            )
        });
        let now = ctx.input(|i| i.time);
        if run {
            self.evaluate(now);
        }
        // auto-evaluation: wait until typing pauses, then run in the background
        self.poll_job();
        let h = doc_hash(&self.doc);
        if h != self.last_change.0 {
            self.last_change = (h, now);
        }
        if h != self.submitted && (self.auto || self.submitted == 0) {
            let wait = 0.3 - (now - self.last_change.1);
            if wait <= 0.0 {
                self.evaluate(now);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
            }
        }
        if self.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
        if save || save_as {
            self.save(save_as);
        }
        if open {
            self.replace_doc(Replace::Open);
        }
        if new {
            self.replace_doc(Replace::New);
        }
        // drag & drop a .sp file onto the window
        if let Some(path) = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .find_map(|f| Some(f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()))
        }) {
            self.replace_doc(Replace::Load(path));
        }
        self.confirm_dialog(&ctx);
        self.paste_dialog(&ctx);
        let (copy, paste) = ctx.input_mut(|i| {
            let cs = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
            (
                i.consume_key(cs, egui::Key::C),
                i.consume_key(cs, egui::Key::V),
            )
        });
        if copy {
            self.copy_project(&ctx);
        }
        if paste {
            self.paste_project();
        }
        // the stepper follows the selected test case
        if self.current != self.traced_test && self.job.is_none() {
            self.submitted = 0;
        }
        if self.results.len() != self.doc.tests.len() {
            self.results.resize(self.doc.tests.len(), None);
        }

        let bar = egui::Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(Margin::symmetric(12, 8));
        egui::Panel::top("toolbar")
            .frame(bar)
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status")
            .frame(bar.inner_margin(Margin::symmetric(12, 3)))
            .show(ui, |ui| self.status_bar(ui));
        let pane = egui::Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(Margin::same(10));
        if self.show_v {
            egui::Panel::bottom("eval")
                .frame(pane)
                .resizable(true)
                .default_size(280.0)
                .show(ui, |ui| self.eval_pane(ui));
        }
        if self.show_s && self.show_d {
            egui::Panel::right("data")
                .frame(pane)
                .resizable(true)
                .default_size(460.0)
                .show(ui, |ui| self.data_pane(ui));
        }
        egui::CentralPanel::default().frame(pane).show(ui, |ui| {
            if self.show_s {
                self.spec_pane(ui);
            } else if self.show_d {
                self.data_pane(ui);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("All editor panes are hidden. Turn one on in the toolbar.")
                            .weak(),
                    )
                });
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

const APP_ID: &str = "speci-local";
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Wayland compositors (KDE, GNOME, …) take a window's icon from the installed `.desktop` file whose
/// name matches the app id, not from the window itself. Install one (plus the icon) for the current
/// user, so the window, taskbar and launcher show the real icon. Only rewritten when it changed.
#[cfg(target_os = "linux")]
fn install_desktop_entry() -> std::io::Result<()> {
    let data = match std::env::var_os("XDG_DATA_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME").ok_or(std::io::ErrorKind::NotFound)?)
            .join(".local/share"),
    };
    let icon = data
        .join("icons/hicolor/256x256/apps")
        .join(format!("{APP_ID}.png"));
    if std::fs::read(&icon).ok().as_deref() != Some(ICON_PNG) {
        std::fs::create_dir_all(icon.parent().unwrap())?;
        std::fs::write(&icon, ICON_PNG)?;
    }
    let exe = std::env::current_exe()?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=speci-local\nComment=Offline ELTE specification editor\nExec=\"{}\" %f\nIcon={APP_ID}\nStartupWMClass={APP_ID}\nTerminal=false\nCategories=Development;Education;\n",
        exe.display()
    );
    let desktop = data.join("applications").join(format!("{APP_ID}.desktop"));
    if std::fs::read_to_string(&desktop).ok().as_deref() != Some(entry.as_str()) {
        std::fs::create_dir_all(desktop.parent().unwrap())?;
        std::fs::write(&desktop, entry)?;
    }
    Ok(())
}

fn main() -> eframe::Result {
    #[cfg(target_os = "linux")]
    if let Err(e) = install_desktop_entry() {
        eprintln!("speci-local: could not install the desktop entry (window icon): {e}");
    }
    let mut viewport = egui::ViewportBuilder::default().with_app_id(APP_ID);
    // X11 and Windows use this directly
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport: viewport
            .with_inner_size([1360.0, 900.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("speci-local"),
        ..Default::default()
    };
    let file_arg = std::env::args_os().nth(1).map(PathBuf::from);
    eframe::run_native(
        "speci-local",
        options,
        Box::new(move |cc| {
            fonts(&cc.egui_ctx);
            let mut app: App = cc
                .storage
                .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
                .unwrap_or_default();
            // `speci-local file.sp` (also how double-clicking a .sp file opens it)
            if let Some(p) = file_arg {
                app.load(&p);
            }
            Ok(Box::new(app))
        }),
    )
}
