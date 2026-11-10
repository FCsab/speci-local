//! Stuki: the structogram (Nassi–Shneiderman diagram) editor, an offline version of
//! <https://progalap.elte.hu/stuki/>.
//!
//! Elements are dragged in from the Elements menu (or inserted next to the selected one with
//! ↑ / ↓), texts are edited in place, and the colors, distances and other appearance settings
//! match the web app. Projects are saved as `.st` files (the web app's own JSON), copied to the
//! clipboard as one `STUKI1:` line, and pictures are copied or exported as PNG.

mod canvas;
mod colors;
mod export;
mod files;
pub mod format;
pub mod layout;
mod menu;
pub mod model;
pub mod png;
pub mod rich;
pub mod settings;
#[cfg(test)]
mod tests;

use eframe::egui::{self, Color32, Frame, Id, Margin, Rect};
use layout::Del;
use model::{Diagram, Node, Place, Program, Tag, TextKey};
use serde::{Deserialize, Serialize};
use settings::{Look, Settings};
use std::path::PathBuf;

const GREEN: Color32 = Color32::from_rgb(0, 128, 0);
const UNDO_LIMIT: usize = 200;

/// Something that replaces the current structogram (asks first if it has unsaved changes).
#[derive(Clone)]
pub enum Pending {
    New,
    Open,
    Load(PathBuf),
    Import(Program),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Drag {
    /// A new element from the Elements menu.
    New(Tag),
    /// An element of a structogram.
    Node(u64),
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    At(u64, Place, Rect),
    Delete,
    Nothing,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Pal {
    Text,
    TextBg,
    Node,
    Diagram,
}

#[derive(Clone, Copy)]
enum Part {
    Header,
    Variables,
    Return,
}

enum Action {
    Del(Del),
    AddCase(u64, bool),
    AddDiagram,
    DelDiagram(u64),
    Toggle(u64, Part),
    Drop(Drag, Target),
    CopyImage(u64),
}

fn text_id(key: TextKey) -> Id {
    Id::unique(("stuki-text", key))
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Stuki {
    pub program: Program,
    /// [`Program::content`] of what is on disk; differs from the program's: unsaved changes.
    saved: String,
    pub path: Option<PathBuf>,
    pub settings: Settings,
    show_colors: bool,
    show_help: bool,
    #[serde(skip)]
    selected: Option<u64>,
    /// Diagram whose header / variables / return text was clicked last.
    #[serde(skip)]
    focus_diagram: Option<u64>,
    /// The text the color buttons color.
    #[serde(skip)]
    last_text: Option<TextKey>,
    /// Text being typed into, already saved for undo (one undo step per editing session).
    #[serde(skip)]
    armed: Option<TextKey>,
    #[serde(skip)]
    undo: Vec<Program>,
    #[serde(skip)]
    redo: Vec<Program>,
    #[serde(skip)]
    drag: Option<Drag>,
    #[serde(skip)]
    editing: Option<(Pal, Vec<String>)>,
    #[serde(skip)]
    confirm: Option<Pending>,
    #[serde(skip)]
    previews: Vec<(Tag, Diagram)>,
    /// The structogram that was right-clicked (its context menu works on it).
    #[serde(skip)]
    menu_diagram: Option<usize>,
    #[serde(skip)]
    pub status: String,
    /// Tests: where the elements (id, rect) and the Elements menu's pictures were drawn.
    #[cfg(test)]
    #[serde(skip)]
    pub(crate) seen: (Vec<(u64, Rect)>, Vec<Rect>),
    /// Android: show the app's own Open (false) / Save as (true) list, with this file name.
    #[serde(skip)]
    pub file_request: Option<(bool, String)>,
    /// What to do once the Android "Save as" list has saved the file.
    #[serde(skip)]
    pub after_save: Option<Pending>,
}

impl Default for Stuki {
    fn default() -> Self {
        let program = Program::default();
        Stuki {
            saved: program.content(),
            program,
            path: None,
            settings: Settings::default(),
            show_colors: false,
            show_help: false,
            selected: None,
            focus_diagram: None,
            last_text: None,
            armed: None,
            undo: vec![],
            redo: vec![],
            drag: None,
            editing: None,
            confirm: None,
            previews: vec![],
            menu_diagram: None,
            status: String::new(),
            #[cfg(test)]
            seen: Default::default(),
            file_request: None,
            after_save: None,
        }
    }
}

impl Stuki {
    pub fn dirty(&self) -> bool {
        !self.program.content_is(&self.saved)
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.st".into())
    }

    fn push_undo(&mut self) {
        self.undo.push(self.program.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.armed = None;
    }

    fn undo(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.program, p));
            self.after_history();
        }
    }

    fn redo(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.program, p));
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.selected = None;
        self.armed = None;
        self.drag = None;
    }

    fn reset(&mut self, program: Program, path: Option<PathBuf>, saved: bool) {
        self.saved = if saved {
            program.content()
        } else {
            String::new()
        };
        self.program = program;
        self.path = path;
        self.undo.clear();
        self.redo.clear();
        self.selected = None;
        self.focus_diagram = None;
        self.last_text = None;
        self.armed = None;
        self.drag = None;
        self.editing = None;
    }

    // ---------------------------------------------------------------- UI

    /// The whole Stuki tab: menu, color panel, help and the editor.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.keyboard(&ctx);
        let bar = Frame::new()
            .fill(ui.visuals().panel_fill)
            .inner_margin(Margin::symmetric(12, 6));
        egui::Panel::top("stuki-menu")
            .resizable(false)
            .frame(bar)
            .show(ui, |ui| self.menu(ui));
        if self.show_colors {
            egui::Panel::top("stuki-colors")
                .resizable(false)
                .frame(bar.fill(ui.visuals().faint_bg_color))
                .show(ui, |ui| self.colors_panel(ui));
        }
        if self.show_help {
            egui::Panel::top("stuki-help")
                .resizable(false)
                .frame(bar.fill(ui.visuals().faint_bg_color))
                .show(ui, menu::help);
        }
        let look = Look::new(ui.visuals().dark_mode);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(look.bg))
            .show(ui, |ui| self.canvas(ui, look));
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.drag = None;
        }
        if ctx.text_edit_focused() {
            return; // the text field has its own undo
        }
        // Ctrl+C (arrives as a copy event) outside the texts: the picture
        if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
            self.copy_image(ctx, None);
        }
        let cmd = egui::Modifiers::COMMAND;
        let (redo, undo, del) = ctx.input_mut(|i| {
            let redo = i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::Z)
                || i.consume_key(cmd, egui::Key::Y);
            let undo = i.consume_key(cmd, egui::Key::Z);
            let del = i.consume_key(egui::Modifiers::NONE, egui::Key::Delete);
            (redo, undo, del)
        });
        if redo {
            self.redo();
        }
        if undo {
            self.undo();
        }
        if del && let Some(id) = self.selected {
            self.push_undo();
            self.program.remove(id);
            self.selected = None;
        }
    }

    fn apply(&mut self, a: Action) {
        match a {
            Action::Del(Del::Node(id)) => {
                self.push_undo();
                self.program.remove(id);
                self.selected = None;
            }
            Action::Del(Del::Case { switch, case }) => {
                self.push_undo();
                self.program.delete_case(switch, case);
                self.selected = None;
            }
            Action::AddCase(id, default) => {
                self.push_undo();
                self.program.add_case(id, default);
            }
            Action::AddDiagram => {
                self.push_undo();
                self.program.diagrams.push(Diagram::new());
            }
            Action::DelDiagram(id) => {
                self.push_undo();
                self.program.diagrams.retain(|d| d.id != id);
                self.selected = None;
            }
            Action::Toggle(id, which) => {
                self.push_undo();
                if let Some(d) = self.program.diagrams.iter_mut().find(|d| d.id == id) {
                    let flag = match which {
                        Part::Header => &mut d.show_header,
                        Part::Variables => &mut d.show_variables,
                        Part::Return => &mut d.show_return,
                    };
                    *flag = !*flag;
                }
            }
            Action::Drop(Drag::New(tag), Target::At(t, place, _)) => {
                self.push_undo();
                let node = Node::new(tag);
                let id = node.id;
                if self.program.insert(t, place, node).is_ok() {
                    self.selected = Some(id);
                }
            }
            Action::Drop(Drag::Node(id), Target::At(t, place, _)) => {
                let snap = self.program.clone();
                if self.program.move_node(id, t, place) {
                    self.undo.push(snap);
                    self.redo.clear();
                    self.armed = None;
                    self.selected = Some(id);
                }
            }
            Action::Drop(Drag::Node(id), Target::Delete) => {
                if self.program.find(id).is_some() {
                    self.push_undo();
                    self.program.remove(id);
                    self.selected = None;
                    self.status = "Deleted (dropped outside the structograms)".into();
                }
            }
            Action::Drop(..) | Action::CopyImage(_) => {}
        }
    }
}
