//! The application window: the specification editor with its test cases and results, and the
//! Stuki tab.

#[cfg(target_os = "android")]
mod android_files;
mod dialogs;
mod files;
mod job;
mod launch;
mod panes;
mod results;
mod stepper;
#[cfg(test)]
mod tests;
mod theme;
mod toolbar;

#[cfg(target_os = "android")]
pub use launch::run_android;
#[cfg(not(target_os = "android"))]
pub use launch::run_desktop;
#[cfg(test)]
pub(crate) use theme::fonts;

use crate::Outcome;
use crate::diag::Diagnostic;
use crate::editor::{Completion, Item};
use crate::eval::Trace;
use crate::file::Doc;
use crate::platform;
use crate::stuki::Stuki;
use eframe::egui::{self, Id, Margin, RichText};
use job::Job;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

const NARROW_BELOW: f32 = 760.0;
const AUTO_DELAY: f64 = 0.3;

/// Something that replaces the current specification (asks first if it has unsaved changes).
#[derive(Clone)]
enum Replace {
    New,
    Open,
    Load(PathBuf),
    Import(Doc),
}

/// The pane shown on narrow (phone) screens, where only one fits.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
enum Tab {
    #[default]
    Spec,
    Data,
    Eval,
}

/// The two editors, each in its own tab.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Default, Debug)]
pub enum Tool {
    /// Specifications and their test cases.
    #[default]
    Spec,
    /// Structograms.
    Stuki,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
enum View {
    #[default]
    Results,
    Stepper,
    StepList,
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
    tab: Tab,
    /// The editor shown: specifications or structograms.
    tool: Tool,
    stuki: Stuki,
    /// Only one pane fits on screen (phones): `tab` picks it.
    #[serde(skip)]
    narrow: bool,
    #[cfg(target_os = "android")]
    #[serde(skip)]
    files: Option<android_files::FileDialog>,
    /// (screen insets, time they last changed): keyboard/system bars on Android
    #[serde(skip)]
    insets: (egui::epaint::MarginF32, f64),
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
            tab: Tab::Spec,
            tool: Tool::Spec,
            stuki: Stuki::default(),
            narrow: false,
            #[cfg(target_os = "android")]
            files: None,
            insets: (egui::epaint::MarginF32::ZERO, 0.0),
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

fn spec_editor_id() -> Id {
    Id::unique("spec-editor")
}

fn data_editor_id(test: usize) -> Id {
    Id::unique(("data-editor", test))
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
    let (mut line, mut col) = (1, 1);
    for c in text.chars().take(idx) {
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

struct Keys {
    run: bool,
    save_as: bool,
    save: bool,
    open: bool,
    new: bool,
}

impl Keys {
    fn take(ctx: &egui::Context) -> Keys {
        use egui::{Key, Modifiers};
        // Ctrl+Shift+S must be checked before Ctrl+S
        ctx.input_mut(|i| Keys {
            run: i.consume_key(Modifiers::COMMAND, Key::Enter),
            save_as: i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S),
            save: i.consume_key(Modifiers::COMMAND, Key::S),
            open: i.consume_key(Modifiers::COMMAND, Key::O),
            new: i.consume_key(Modifiers::COMMAND, Key::N),
        })
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.sync_window(&ctx);

        let keys = Keys::take(&ctx);
        if keys.run && self.tool == Tool::Spec {
            self.evaluate(now);
        }
        self.poll_job();
        self.auto_evaluate(&ctx, now);
        if keys.save || keys.save_as {
            self.file_save(keys.save_as);
        }
        if keys.open {
            self.file_open();
        }
        if keys.new {
            self.file_new();
        }
        self.dropped_file(&ctx);
        self.dialogs(&ctx);
        self.clipboard_shortcuts(&ctx);

        // the stepper follows the selected test case
        self.current = self.current.min(self.doc.tests.len().saturating_sub(1));
        if self.current != self.traced_test && self.job.is_none() {
            self.submitted = 0;
        }
        self.results.resize(self.doc.tests.len(), None);

        let insets = self.screen_insets(&ctx, now);
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        let area = ui.max_rect() - insets;
        self.narrow = area.width() < NARROW_BELOW;
        ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| self.layout(ui));

        // Android: egui's own clipboard stays inside the app, so pass copies on to the system
        #[cfg(target_os = "android")]
        for text in ctx.output(|o| {
            o.commands
                .iter()
                .filter_map(|c| match c {
                    egui::OutputCommand::CopyText(t) => Some(t.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        }) {
            platform::android::set_clipboard(&text);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

impl App {
    fn sync_window(&mut self, ctx: &egui::Context) {
        if self.applied.0 != self.font {
            theme::theme(ctx, self.font);
            self.applied.0 = self.font;
        }
        let (name, dirty) = match self.tool {
            Tool::Stuki => (self.stuki.file_name(), self.stuki.dirty()),
            Tool::Spec => (
                self.path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "untitled.sp".into()),
                self.dirty(),
            ),
        };
        let title = format!("{}{name} — speci-local", if dirty { "● " } else { "" });
        if self.applied.1 != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.applied.1 = title;
        }
    }

    /// Auto-evaluation: wait until typing pauses, then run in the background.
    fn auto_evaluate(&mut self, ctx: &egui::Context, now: f64) {
        let h = job::doc_hash(&self.doc);
        if h != self.last_change.0 {
            self.last_change = (h, now);
        }
        if h != self.submitted && (self.auto || self.submitted == 0) {
            let wait = AUTO_DELAY - (now - self.last_change.1);
            if wait <= 0.0 {
                self.evaluate(now);
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(wait));
            }
        }
        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }

    /// Drag & drop of a `.sp` or `.st` file onto the window.
    fn dropped_file(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .find_map(|f| Some(f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()))
        });
        if let Some(path) = dropped {
            self.open_path(path);
        }
    }

    fn clipboard_shortcuts(&mut self, ctx: &egui::Context) {
        let (copy, paste) = ctx.input_mut(|i| {
            let cs = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
            (
                i.consume_key(cs, egui::Key::C),
                i.consume_key(cs, egui::Key::V),
            )
        });
        if copy {
            match self.tool {
                Tool::Spec => self.copy_project(ctx),
                Tool::Stuki => self.stuki.copy_project(ctx),
            }
        }
        if paste {
            self.paste_project();
        }
    }

    /// Android: the parts of the screen under the status bar, navigation bar and on-screen keyboard.
    fn screen_insets(&mut self, ctx: &egui::Context, now: f64) -> egui::epaint::MarginF32 {
        let insets = platform::insets(ctx);
        if insets != self.insets.0 {
            self.insets = (insets, now);
        }
        if platform::MOBILE && (ctx.memory(|m| m.focused().is_some()) || now - self.insets.1 < 1.0)
        {
            // Android doesn't tell the app when the keyboard opens or closes: look again soon
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        insets
    }

    fn layout(&mut self, ui: &mut egui::Ui) {
        let bar = egui::Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(Margin::symmetric(12, 8));
        egui::Panel::top("toolbar")
            .frame(bar)
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status")
            .frame(bar.inner_margin(Margin::symmetric(12, 3)))
            .show(ui, |ui| self.status_bar(ui));
        if self.tool == Tool::Stuki {
            self.stuki.show(ui);
            return;
        }
        let pane = egui::Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(Margin::same(10));
        if self.narrow {
            egui::CentralPanel::default()
                .frame(pane)
                .show(ui, |ui| match self.tab {
                    Tab::Spec => self.spec_pane(ui),
                    Tab::Data => self.data_pane(ui),
                    Tab::Eval => self.eval_pane(ui),
                });
            return;
        }
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
}
