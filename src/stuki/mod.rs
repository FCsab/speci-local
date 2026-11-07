//! Stuki: the structogram (Nassi–Shneiderman diagram) editor, an offline version of
//! <https://progalap.elte.hu/stuki/>.
//!
//! Elements are dragged in from the Elements menu (or inserted next to the selected one with
//! ↑ / ↓), texts are edited in place, and the colors, distances and other appearance settings
//! match the web app. Projects are saved as `.st` files (the web app's own JSON), copied to the
//! clipboard as one `STUKI1:` line, and pictures are copied or exported as PNG.

pub mod layout;
pub mod model;
pub mod png;
pub mod rich;

use crate::platform;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, Frame, Id, Margin, Pos2, Rect, RichText,
    Sense, Stroke, StrokeKind, TextEdit, Vec2, pos2, vec2,
};
use layout::{Del, Lay, Look, Settings, css_color, rich_job};
use model::{Diagram, Kind, Node, Place, Program, Tag, TextKey};
use rich::{Rich, hex, parse_color};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const I_UNDO: &str = "\u{f0e2}";
const I_REDO: &str = "\u{f01e}";
const I_IMAGE: &str = "\u{f03e}";
const I_DOWNLOAD: &str = "\u{f019}";
const I_HELP: &str = "\u{f059}";
const I_CLOSE: &str = "\u{f00d}";
const I_MOVE: &str = "\u{f047}";
const I_BRUSH: &str = "\u{f1fc}";
const I_SAVE: &str = "\u{f0c7}";
const I_SLIDERS: &str = "\u{f1de}";
const I_TEXT: &str = "\u{f0f6}";
const RED: Color32 = Color32::from_rgb(229, 72, 77);
const GREEN: Color32 = Color32::from_rgb(0, 128, 0);
const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);

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

enum Action {
    Del(Del),
    AddCase(u64, bool),
    AddDiagram,
    DelDiagram(u64),
    /// 0: header, 1: variables, 2: return value
    Toggle(u64, u8),
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
        self.program.content() != self.saved
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
        if self.undo.len() > 200 {
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

    // ---------------------------------------------------------------- files

    /// Run `action`, first asking what to do with unsaved changes.
    pub fn request(&mut self, action: Pending) {
        if self.dirty() {
            self.confirm = Some(action);
        } else {
            self.run(action);
        }
    }

    fn run(&mut self, action: Pending) {
        match action {
            Pending::New => {
                self.reset(Program::default(), None, true);
                self.status = "New structogram".into();
            }
            Pending::Open => {
                #[cfg(not(target_os = "android"))]
                if let Some(p) = platform::pick_open(platform::FileKind::Stuki) {
                    self.load(&p);
                }
                #[cfg(target_os = "android")]
                {
                    self.file_request = Some((false, String::new()));
                }
            }
            Pending::Load(p) => self.load(&p),
            Pending::Import(p) => {
                self.reset(p, None, false);
                self.status =
                    "Structogram read from the clipboard (not saved to a file yet)".into();
            }
        }
    }

    pub fn load(&mut self, path: &Path) {
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| model::from_st(&t))
        {
            Ok(p) => {
                self.reset(p, Some(path.to_path_buf()), true);
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    /// File name to suggest: the current file's, or the first structogram's name.
    fn suggested_name(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .or_else(|| {
                let n = self.program.diagrams.first()?.name.text.trim().to_string();
                (!n.is_empty()).then_some(n)
            })
            .unwrap_or_else(|| "structogram".into());
        name.chars()
            .map(|c| if "/\\:*?\"<>|\n".contains(c) { '_' } else { c })
            .collect()
    }

    pub fn save(&mut self, as_new: bool) {
        match (&self.path, as_new) {
            (Some(p), false) => self.save_to(p.clone()),
            _ => {
                let name = self.suggested_name();
                #[cfg(not(target_os = "android"))]
                if let Some(p) = platform::pick_save(&name, platform::FileKind::Stuki) {
                    self.save_to(p);
                }
                #[cfg(target_os = "android")]
                {
                    self.file_request = Some((true, name));
                }
            }
        }
    }

    pub fn save_to(&mut self, mut path: PathBuf) {
        if path.extension().is_none_or(|e| e != "st") {
            path.set_extension("st");
        }
        match std::fs::write(&path, model::to_st(&self.program)) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.saved = self.program.content();
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not save {}: {e}", path.display()),
        }
    }

    pub fn copy_project(&mut self, ctx: &egui::Context) {
        let text = model::to_compact(&self.program);
        self.status = format!(
            "Copied the whole structogram project to the clipboard ({} characters)",
            text.len()
        );
        ctx.copy_text(text);
    }

    /// The "unsaved changes" dialog.
    pub fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else {
            return;
        };
        let mut choice = None;
        let modal = egui::Modal::new(Id::unique("stuki-discard-dialog")).show(ctx, |ui| {
            ui.set_width(420.0f32.min(ctx.content_rect().width() - 48.0).max(200.0));
            ui.heading("Unsaved changes");
            ui.label("The current structogram has changes that are not saved yet.");
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(format!("{I_SAVE}  Save first")).clicked() {
                    choice = Some(0);
                }
                if ui.button("Discard changes").clicked() {
                    choice = Some(1);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(2);
                }
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some(2);
        }
        match choice {
            Some(0) => {
                self.confirm = None;
                self.save(false);
                if !self.dirty() {
                    self.run(action);
                } else if self.file_request.is_some() {
                    self.after_save = Some(action); // Android: continue once saved
                }
            }
            Some(1) => {
                self.confirm = None;
                self.run(action);
            }
            Some(_) => self.confirm = None,
            None => {}
        }
    }

    // ---------------------------------------------------------------- PNG

    /// Index of the structogram the PNG buttons and the background colors work on.
    fn current_diagram(&self) -> usize {
        self.selected
            .and_then(|s| self.program.diagram_of(s))
            .or_else(|| {
                let d = self.focus_diagram?;
                self.program.diagrams.iter().position(|x| x.id == d)
            })
            .unwrap_or(0)
    }

    /// Structogram `index` (default: the selected one) as a picture.
    pub(crate) fn render_png(&self, ctx: &egui::Context, index: Option<usize>) -> Option<egui::ColorImage> {
        let d = self.program.diagrams.get(index.unwrap_or_else(|| self.current_diagram()))?;
        let ppp = ctx.pixels_per_point();
        // at least twice the font size in pixels, so the picture stays sharp when scaled
        let scale = (2.0 / ppp).max(1.0);
        let dark = ctx.global_style().visuals.dark_mode && !self.settings.light_pictures;
        let look = Look::new(dark);
        let settings = self.settings.clone();
        let mut lay = Lay::new(ctx, &settings, look, scale);
        lay.hints = false;
        let size = lay.diagram_size(d);
        let margin = (settings.border * lay.em()).max(1.0);
        lay.place_diagram(d, pos2(margin, margin));
        let bg = d.bg.as_deref().and_then(css_color).unwrap_or(look.bg);
        Some(png::render(
            ctx,
            lay.out.shapes(true),
            size + Vec2::splat(2.0 * margin),
            ppp,
            bg,
        ))
    }

    /// Copy structogram `index` (default: the selected one) to the clipboard as a picture.
    pub fn copy_image(&mut self, ctx: &egui::Context, index: Option<usize>) {
        let Some(img) = self.render_png(ctx, index) else {
            return;
        };
        if platform::MOBILE {
            // Android's clipboard only takes images as files shared by a content provider
            self.export_to_folder(&img, "Android can't copy pictures to the clipboard: saved");
            return;
        }
        self.status = format!(
            "Copied the structogram as a {}×{} picture: paste it with {}",
            img.size[0],
            img.size[1],
            platform::shortcut("Ctrl+V")
        );
        ctx.copy_image(img);
    }

    fn export_png(&mut self, ctx: &egui::Context, index: Option<usize>) {
        let Some(img) = self.render_png(ctx, index) else {
            return;
        };
        #[cfg(not(target_os = "android"))]
        {
            let name = self.suggested_name();
            if let Some(mut path) = platform::pick_save(&name, platform::FileKind::Png) {
                if path
                    .extension()
                    .is_none_or(|e| !e.eq_ignore_ascii_case("png"))
                {
                    path.set_extension("png");
                }
                self.write_png(&img, &path, "Exported");
            }
        }
        #[cfg(target_os = "android")]
        self.export_to_folder(&img, "Exported");
    }

    fn export_to_folder(&mut self, img: &egui::ColorImage, what: &str) {
        match platform::documents_dir() {
            Some(dir) => {
                let path = dir.join(format!("{}.png", self.suggested_name()));
                self.write_png(img, &path, what);
            }
            None => self.status = "There is no folder to save the picture in".into(),
        }
    }

    fn write_png(&mut self, img: &egui::ColorImage, path: &Path, what: &str) {
        self.status = match png::encode(img)
            .and_then(|b| std::fs::write(path, b).map_err(|e| e.to_string()))
        {
            Ok(()) => format!("{what} {}", path.display()),
            Err(e) => format!("Could not save {}: {e}", path.display()),
        };
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
                .show(ui, help);
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

    fn menu(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let narrow = ui.available_width() < 700.0;
        let label = |icon: &str, text: &str| {
            if narrow {
                icon.to_string()
            } else {
                format!("{icon}  {text}")
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new(I_UNDO))
                .on_hover_text(platform::shortcut("Undo (Ctrl+Z)"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new(I_REDO))
                .on_hover_text(platform::shortcut("Redo (Ctrl+Y)"))
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            if ui
                .button(label(I_IMAGE, "Copy image"))
                .on_hover_text(platform::shortcut(
                    "Copy the selected structogram to the clipboard as a picture (Ctrl+C)\n\
                     Right-click a structogram to copy that one",
                ))
                .clicked()
            {
                self.copy_image(&ctx, None);
            }
            if ui
                .button(label(I_DOWNLOAD, "Export PNG"))
                .on_hover_text("Save the selected structogram as a PNG picture")
                .clicked()
            {
                self.export_png(&ctx, None);
            }
            if ui
                .button(label(I_TEXT, "Copy as text"))
                .on_hover_text(platform::shortcut(
                    "Copy the whole project as one STUKI1: text line, to paste it back into speci-local (Ctrl+Shift+C)",
                ))
                .clicked()
            {
                self.copy_project(&ctx);
            }
            ui.separator();
            ui.toggle_value(&mut self.settings.colorized, "Colorize")
                .on_hover_text("Color the elements by kind");
            ui.toggle_value(&mut self.show_colors, label(I_BRUSH, "Colors"))
                .on_hover_text("Show / hide the text and background colors");
            if narrow {
                ui.add(
                    egui::DragValue::new(&mut self.settings.zoom)
                        .range(16.0..=64.0)
                        .speed(0.5)
                        .prefix("Zoom "),
                );
            } else {
                ui.add(
                    egui::Slider::new(&mut self.settings.zoom, 16.0..=64.0)
                        .step_by(2.0)
                        .text("Zoom"),
                );
            }
            self.advanced_menu(ui);
            ui.toggle_value(&mut self.show_help, I_HELP)
                .on_hover_text("Help");
        });
        ui.add_space(2.0);
        let look = Look::new(ui.visuals().dark_mode);
        if self.previews.is_empty() {
            self.previews = Tag::ALL.iter().map(|t| (*t, preview(*t))).collect();
        }
        // all entries as tall as the tallest, so they line up
        let tallest = {
            let settings = self.preview_settings();
            let mut lay = Lay::new(&ctx, &settings, look, 1.0);
            self.previews
                .iter()
                .map(|(_, d)| lay.diagram_size(d).y)
                .fold(0.0, f32::max)
        };
        let mut elements = |ui: &mut egui::Ui| {
            ui.label(RichText::new("Elements").small().weak())
                .on_hover_text("Drag them into a structogram, or insert them above (↑) or below (↓) the selected element");
            for i in 0..self.previews.len() {
                self.element(ui, i, look, tallest);
            }
        };
        if narrow {
            // phones: one row to swipe through
            egui::ScrollArea::horizontal()
                .id_salt("stuki-elements")
                .show(ui, |ui| ui.horizontal(|ui| elements(ui)));
        } else {
            ui.horizontal_wrapped(|ui| elements(ui));
        }
    }

    fn advanced_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(I_SLIDERS, |ui| {
            let s = &mut self.settings;
            ui.label(RichText::new("Appearance").strong());
            egui::ComboBox::from_label("Font")
                .selected_text(if s.proportional {
                    "Proportional"
                } else {
                    "Monospace"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.proportional, false, "Monospace");
                    ui.selectable_value(&mut s.proportional, true, "Proportional");
                });
            ui.separator();
            ui.label(RichText::new("Advanced settings").strong());
            ui.horizontal(|ui| {
                ui.label("Distances: hor");
                let h = ui.add(egui::DragValue::new(&mut s.hdist).range(0..=50));
                ui.checkbox(&mut s.linked, "⧉")
                    .on_hover_text("Change both together");
                ui.label("ver");
                let v = ui.add(egui::DragValue::new(&mut s.vdist).range(0..=50));
                if s.linked {
                    if v.changed() && !h.changed() {
                        s.hdist = s.vdist;
                    } else {
                        s.vdist = s.hdist;
                    }
                }
            });
            ui.checkbox(&mut s.snapping, "Snapping (no distance under conditions)");
            ui.horizontal(|ui| {
                ui.label("Block height");
                ui.add(egui::DragValue::new(&mut s.block_height).range(0..=25));
                ui.label("Spacing");
                ui.add(egui::DragValue::new(&mut s.spacing).range(0..=50));
            });
            ui.add(
                egui::Slider::new(&mut s.border, 0.1..=0.2)
                    .step_by(0.005)
                    .text("Border width"),
            );
            ui.checkbox(&mut s.right_padding, "Right padding (in function frames)");
            ui.checkbox(&mut s.light_pictures, "Light pictures (white background, also in dark mode)");
            if ui.button("Reset to defaults").clicked() {
                *s = Settings {
                    colorized: s.colorized,
                    ..Settings::default()
                };
            }
        })
        .response
        .on_hover_text("Font and advanced settings");
    }

    /// One entry of the Elements menu: a small picture to drag, its name, and ↑ / ↓.
    fn preview_settings(&self) -> Settings {
        Settings {
            zoom: 10.0,
            colorized: self.settings.colorized,
            proportional: self.settings.proportional,
            ..Settings::default()
        }
    }

    fn element(&mut self, ui: &mut egui::Ui, i: usize, look: Look, tallest: f32) {
        let settings = self.preview_settings();
        let (tag, d) = &self.previews[i];
        let tag = *tag;
        let ctx = ui.ctx().clone();
        let mut add = None;
        let mut lay = Lay::new(&ctx, &settings, look, 1.0);
        let size = lay.diagram_size(d);
        // one fixed-size widget, so the Elements row wraps on narrow screens
        let desc = egui::WidgetText::from(RichText::new(tag.description()).small()).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        );
        let btn = desc.size().y + 8.0;
        let row = vec2(2.0 * btn + 8.0 + desc.size().x, btn);
        let pad = 5.0;
        let outer = vec2(size.x.max(row.x), tallest + 4.0 + row.y) + Vec2::splat(2.0 * pad);
        let (rect, _) = ui.allocate_exact_size(outer, Sense::hover());
        ui.painter().rect_stroke(
            rect,
            4.0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            StrokeKind::Inside,
        );
        let inner = rect.shrink(pad);
        let pic = Rect::from_min_size(inner.min, size);
        let resp = ui.interact(pic, Id::unique(("stuki-element", i)), Sense::drag());
        #[cfg(test)]
        {
            if i == 0 {
                self.seen.1.clear();
            }
            self.seen.1.push(pic);
        }
        lay.place_diagram(d, pic.min);
        ui.painter().rect_filled(pic, 0.0, look.bg);
        ui.painter().extend(lay.out.shapes(true));
        if resp.drag_started() {
            self.drag = Some(Drag::New(tag));
        }
        resp.on_hover_cursor(CursorIcon::Grab)
            .on_hover_text(format!("Drag a {} into a structogram", tag.description()));
        let row_rect = Rect::from_min_size(pos2(inner.left(), inner.top() + tallest + 4.0), row);
        // a child that doesn't take up room of its own: the space is already allocated above
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        {
            let ui = &mut child;
            ui.spacing_mut().item_spacing.x = 2.0;
            let arrow = |t| egui::Button::new(RichText::new(t).small());
            if ui
                .add_sized([btn, btn], arrow("↑"))
                .on_hover_text("Insert above the selected element")
                .clicked()
            {
                add = Some(false);
            }
            if ui
                .add_sized([btn, btn], arrow("↓"))
                .on_hover_text("Insert below the selected element")
                .clicked()
            {
                add = Some(true);
            }
            ui.add_space(2.0);
            ui.label(RichText::new(tag.description()).small());
        }
        if let Some(after) = add {
            match self.selected {
                Some(sel) if self.program.find(sel).is_some() => {
                    self.push_undo();
                    self.selected = self.program.add_next_to(sel, tag, after);
                }
                _ => self.status = "Select an element of a structogram first (click it)".into(),
            }
        }
    }

    fn colors_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (n, (pal, label)) in [
                (Pal::Text, "Text color"),
                (Pal::TextBg, "Text background"),
                (Pal::Node, "Element background"),
                (Pal::Diagram, "Structogram background"),
            ]
            .into_iter()
            .enumerate()
            {
                if n > 0 {
                    ui.separator();
                }
                self.palette(ui, pal, label);
            }
        });
    }

    fn colors_of(&mut self, pal: Pal) -> &mut Vec<String> {
        let c = &mut self.program.colors;
        match pal {
            Pal::Text => &mut c.text,
            Pal::TextBg => &mut c.text_background,
            Pal::Node => &mut c.node_background,
            Pal::Diagram => &mut c.structogram_background,
        }
    }

    fn palette(&mut self, ui: &mut egui::Ui, pal: Pal, label: &str) {
        let tip = match pal {
            Pal::Text | Pal::TextBg => {
                "Colors the selected part of the text you clicked into last (all of it if nothing is selected)"
            }
            Pal::Node => "Colors the selected element",
            Pal::Diagram => "Colors the structogram of the selected element",
        };
        ui.vertical(|ui| {
            ui.label(RichText::new(label).small()).on_hover_text(tip);
            if let Some((p, list)) = &mut self.editing
                && *p == pal
            {
                let mut op = None;
                for (i, c) in list.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let mut rgb = css_color(c).map_or([0, 0, 0], |c| [c.r(), c.g(), c.b()]);
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            *c = hex(u32::from(rgb[0]) << 16
                                | u32::from(rgb[1]) << 8
                                | u32::from(rgb[2]));
                        }
                        if ui.small_button("↑").clicked() {
                            op = Some((i, 0));
                        }
                        if ui.small_button("↓").clicked() {
                            op = Some((i, 1));
                        }
                        if ui.small_button(RichText::new(I_CLOSE).color(RED)).clicked() {
                            op = Some((i, 2));
                        }
                    });
                }
                match op {
                    Some((i, 0)) if i > 0 => list.swap(i, i - 1),
                    Some((i, 1)) if i + 1 < list.len() => list.swap(i, i + 1),
                    Some((i, 2)) => {
                        list.remove(i);
                    }
                    _ => {}
                }
                let mut done = false;
                ui.horizontal(|ui| {
                    if ui.small_button("Add color").clicked() {
                        list.push("#000000".into());
                    }
                    done = ui.small_button("Save and exit").clicked();
                });
                if done {
                    let list = list.clone();
                    self.editing = None;
                    self.push_undo();
                    *self.colors_of(pal) = list;
                }
                return;
            }
            let colors = self.colors_of(pal).clone();
            let mut chosen: Option<Option<String>> = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                if swatch(ui, None, &format!("{label}: none")).on_hover_text("No color").clicked() {
                    chosen = Some(None);
                }
                for c in &colors {
                    if swatch(ui, css_color(c), &format!("{label}: {c}")).on_hover_text(c).clicked() {
                        chosen = Some(Some(c.clone()));
                    }
                }
                if ui.small_button("Edit").clicked() {
                    self.editing = Some((pal, colors.clone()));
                }
            });
            if let Some(c) = chosen {
                self.apply_color(ui.ctx(), pal, c);
            }
        });
    }

    fn apply_color(&mut self, ctx: &egui::Context, pal: Pal, color: Option<String>) {
        match pal {
            Pal::Text | Pal::TextBg => {
                let Some(key) = self.last_text.filter(|k| self.program.text(*k).is_some()) else {
                    self.status =
                        "Click into a text first (select a part of it to color only that part)"
                            .into();
                    return;
                };
                let id = text_id(key);
                let (a, b) = egui::text_edit::TextEditState::load(ctx, id)
                    .and_then(|s| s.cursor.char_range())
                    .map_or((0, 0), |r| {
                        (r.primary.index.into(), r.secondary.index.into())
                    });
                let c = color.as_deref().and_then(parse_color);
                self.push_undo();
                if let Some(t) = self.program.text_mut(key) {
                    t.paint(a, b, |s| {
                        if pal == Pal::Text {
                            s.color = c
                        } else {
                            s.bg = c
                        }
                    });
                }
                ctx.memory_mut(|m| m.request_focus(id));
            }
            Pal::Node => {
                let Some(id) = self.selected.filter(|s| self.program.find(*s).is_some()) else {
                    self.status = "Select an element first (click it)".into();
                    return;
                };
                self.push_undo();
                if let Some(n) = self.program.find_mut(id) {
                    n.bg = color;
                }
            }
            Pal::Diagram => {
                let i = self.current_diagram();
                if i < self.program.diagrams.len() {
                    self.push_undo();
                    self.program.diagrams[i].bg = color;
                }
            }
        }
    }

    // ---------------------------------------------------------------- the editor

    fn canvas(&mut self, ui: &mut egui::Ui, look: Look) {
        let ctx = ui.ctx().clone();
        if ui.ui_contains_pointer() {
            // pinch or Ctrl+wheel
            let z = ui.input(|i| i.zoom_delta());
            if z != 1.0 {
                self.settings.zoom = (self.settings.zoom * z).clamp(16.0, 64.0);
            }
        }
        let settings = self.settings.clone();
        let mut actions = Vec::new();
        egui::ScrollArea::both()
            .id_salt("stuki-canvas")
            .auto_shrink(false)
            .show(ui, |ui| {
                if let Some(t) = ui.input(|i| i.multi_touch()) {
                    // two fingers move the editor (one finger drags elements)
                    ui.scroll_with_delta(t.translation_delta);
                    if matches!(self.drag, Some(Drag::Node(_))) {
                        self.drag = None;
                    }
                }
                let viewport = ui.clip_rect();
                let mut lay = Lay::new(&ctx, &settings, look, 1.0);
                lay.selected = self.selected;
                let f = lay.em();
                let bw = settings.border * f;
                let (gap, pad) = (1.25 * f, 0.5 * f);
                let width = ui.available_width();
                let sizes: Vec<Vec2> = self
                    .program
                    .diagrams
                    .iter()
                    .map(|d| lay.diagram_size(d))
                    .collect();
                let plus = Vec2::splat(2.2 * f);
                let mut pos = Vec::new();
                let (mut x, mut y, mut row, mut right) = (pad, pad + gap, 0.0f32, 0.0f32);
                for s in sizes.iter().chain([&plus]) {
                    if x > pad && x + s.x > width - pad {
                        x = pad;
                        y += row + 1.5 * gap;
                        row = 0.0;
                    }
                    pos.push(vec2(x, y));
                    right = right.max(x + s.x);
                    x += s.x + gap;
                    row = row.max(s.y);
                }
                let total = vec2(right + pad, y + row + pad + gap).max(ui.available_size());
                let (rect, resp) = ui.allocate_exact_size(total, Sense::click_and_drag());
                for (d, p) in self.program.diagrams.iter().zip(&pos) {
                    lay.place_diagram(d, rect.min + *p);
                }
                let out = std::mem::take(&mut lay.out);
                #[cfg(test)]
                {
                    self.seen.0 = out.hits.iter().map(|h| (h.id, h.rect)).collect();
                }
                ui.painter().extend(out.shapes(false));

                let pointer = ui
                    .input(|i| i.pointer.latest_pos())
                    .filter(|p| viewport.contains(*p));
                let over = pointer.filter(|_| ui.ui_contains_pointer() || self.drag.is_some());
                let hit_at = |p: Pos2| out.hits.iter().rev().find(|h| h.rect.contains(p));
                if self.drag.is_none()
                    && let Some(h) = over.and_then(hit_at)
                {
                    ui.painter().rect_stroke(
                        h.rect,
                        0.0,
                        Stroke::new(bw.max(1.5), layout::HOVER),
                        StrokeKind::Middle,
                    );
                }

                // the "+" tile
                let plus_r = Rect::from_min_size(rect.min + *pos.last().unwrap(), plus);
                let b = egui::Button::new(RichText::new("+").size(1.6 * f).color(GREEN))
                    .fill(GREEN.gamma_multiply(0.12))
                    .stroke(Stroke::new(1.0, GREEN.gamma_multiply(0.6)))
                    .corner_radius(CornerRadius::same((0.6 * f) as u8));
                if ui
                    .put(plus_r, b)
                    .on_hover_text("Add new structogram")
                    .clicked()
                {
                    actions.push(Action::AddDiagram);
                }

                for tb in &out.texts {
                    self.text_edit(ui, tb);
                }

                if self.drag.is_none()
                    && let Some(p) = over
                {
                    self.hover_buttons(ui, &out, p, f, bw, &mut actions);
                }

                if resp.clicked() {
                    self.selected = pointer.and_then(hit_at).map(|h| h.id);
                    if let Some(p) = pointer
                        && let Some((d, _)) = out.diagrams.iter().find(|(_, r)| r.contains(p))
                    {
                        self.focus_diagram = Some(*d);
                    }
                }
                // right-click a structogram: copy / export that one
                if resp.secondary_clicked() {
                    self.menu_diagram = pointer.and_then(|p| out.diagrams.iter().position(|(_, r)| r.contains(p)));
                }
                resp.context_menu(|ui| {
                    let i = self.menu_diagram;
                    if ui.button(format!("{I_IMAGE}  Copy image")).clicked() {
                        self.copy_image(ui.ctx(), i);
                        ui.close();
                    }
                    if ui.button(format!("{I_DOWNLOAD}  Export PNG…")).clicked() {
                        ui.close();
                        self.export_png(ui.ctx(), i);
                    }
                    ui.separator();
                    if ui.button(format!("{I_TEXT}  Copy project as text")).clicked() {
                        self.copy_project(ui.ctx());
                        ui.close();
                    }
                });
                if resp.drag_started()
                    && self.drag.is_none()
                    && let Some(o) = ui.input(|i| i.pointer.press_origin())
                    && let Some(h) = hit_at(o)
                {
                    self.drag = Some(Drag::Node(h.id));
                }

                if let Some(drag) = self.drag {
                    let target = self.drop_target(&out, drag, pointer);
                    let painter = ui.painter();
                    let thick = (1.5 * bw).max(3.0);
                    match target {
                        Target::At(_, Place::Replace, r) => {
                            painter.rect_filled(r, 0.0, ACCENT.gamma_multiply(0.35));
                        }
                        Target::At(_, place, r) => {
                            let y = if place == Place::Before {
                                r.top()
                            } else {
                                r.bottom()
                            };
                            painter.line_segment(
                                [pos2(r.left(), y), pos2(r.right(), y)],
                                Stroke::new(thick, ACCENT),
                            );
                        }
                        Target::Delete => {
                            if let Drag::Node(id) = drag
                                && let Some(h) = out.hits.iter().find(|h| h.id == id)
                            {
                                painter.rect_filled(h.rect, 0.0, RED.gamma_multiply(0.35));
                            }
                        }
                        Target::Nothing => {}
                    }
                    let released = ui.input(|i| i.pointer.any_released() || !i.pointer.any_down());
                    if released {
                        actions.push(Action::Drop(drag, target));
                        self.drag = None;
                    }
                }
            });
        if let Some(drag) = self.drag {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
            if let Some(p) = ctx.pointer_latest_pos() {
                let what = match drag {
                    Drag::New(t) => t.description().to_string(),
                    Drag::Node(id) => match self.program.find(id).map(|n| &n.kind) {
                        Some(Kind::Statement(t)) if !t.text.is_empty() => t.text.clone(),
                        Some(Kind::Statement(_)) => "statement".into(),
                        Some(Kind::Noop) => "-".into(),
                        Some(Kind::If { .. }) => "decision".into(),
                        Some(Kind::While { .. }) => "pre-test loop".into(),
                        Some(Kind::Repeat { .. }) => "post-test loop".into(),
                        Some(Kind::Switch { .. }) => "multi-way decision".into(),
                        None => String::new(),
                    },
                };
                egui::Area::new(Id::unique("stuki-ghost"))
                    .fixed_pos(p + vec2(14.0, 10.0))
                    .order(egui::Order::Tooltip)
                    .interactable(false)
                    .show(&ctx, |ui| {
                        Frame::popup(ui.style()).show(ui, |ui| ui.label(what));
                    });
            }
        }
        for a in actions {
            match a {
                Action::CopyImage(id) => {
                    let i = self.program.diagrams.iter().position(|d| d.id == id);
                    self.copy_image(&ctx, i);
                }
                a => self.apply(a),
            }
        }
    }

    fn drop_target(&self, out: &layout::Out, drag: Drag, pointer: Option<Pos2>) -> Target {
        let Some(p) = pointer else {
            return if matches!(drag, Drag::Node(_)) {
                Target::Delete
            } else {
                Target::Nothing
            };
        };
        let moving: HashSet<u64> = match drag {
            Drag::Node(id) => {
                let mut s = HashSet::new();
                if let Some(n) = self.program.find(id) {
                    collect_ids(n, &mut s);
                }
                s
            }
            Drag::New(_) => HashSet::new(),
        };
        let em = self.settings.zoom;
        // a band along the top / bottom edge of an element
        let band = (0.35 * em).max(6.0);
        let at = |i: usize, place: Place| {
            let h = &out.hits[i];
            if moving.contains(&h.id) {
                Target::Nothing
            } else if h.noop {
                Target::At(h.id, Place::Replace, h.rect)
            } else {
                Target::At(h.id, place, h.rect)
            }
        };
        if let Some(i) = out.hits.iter().rposition(|h| h.rect.contains(p)) {
            let h = &out.hits[i];
            if moving.contains(&h.id) {
                return Target::Nothing;
            }
            // the bottom edge of a block's last element is also the bottom of the loop or
            // decision around it: dropping there puts the element after that one
            if p.y > h.rect.bottom() - band
                && h.last
                && let Some(parent) = h.parent
            {
                return at(parent, Place::After);
            }
            if p.y < h.rect.top() + band
                && h.first
                && let Some(parent) = h.parent
            {
                return at(parent, Place::Before);
            }
            let place = if p.y < h.grip.center().y {
                Place::Before
            } else {
                Place::After
            };
            return at(i, place);
        }
        // just below a structogram: after its last element
        for (rect, body, last) in &out.ends {
            let below = rect.left() - band <= p.x
                && p.x <= rect.right() + band
                && p.y >= body.bottom()
                && p.y <= rect.bottom().max(body.bottom()) + 2.0 * em;
            if below {
                return at(*last, Place::After);
            }
        }
        if matches!(drag, Drag::Node(_)) && !out.diagrams.iter().any(|(_, r)| r.contains(p)) {
            Target::Delete
        } else {
            Target::Nothing
        }
    }

    /// The ✕ and move handle of the hovered cell, the switch's +C / +D, and the ✕ F V R bar.
    fn hover_buttons(
        &mut self,
        ui: &mut egui::Ui,
        out: &layout::Out,
        p: Pos2,
        f: f32,
        bw: f32,
        actions: &mut Vec<Action>,
    ) {
        let s = (0.8 * f).clamp(if platform::MOBILE { 24.0 } else { 15.0 }, 28.0);
        let icon = |text: &str, color: Color32| {
            egui::Button::new(RichText::new(text).size(s * 0.7).color(color))
                .frame(false)
                .min_size(Vec2::splat(s))
        };
        if let Some(cell) = out.cells.iter().rev().find(|c| c.rect.contains(p)) {
            let mut x = cell.rect.right() - bw / 2.0 - 1.0;
            let top = cell.rect.top() + bw / 2.0 + 1.0;
            if let Some(del) = cell.del {
                let r = Rect::from_min_size(pos2(x - s, top), Vec2::splat(s));
                let tip = match del {
                    Del::Case { .. } => "Delete this case",
                    Del::Node(_) => "Delete",
                };
                if ui.put(r, icon(I_CLOSE, RED)).on_hover_text(tip).clicked() {
                    actions.push(Action::Del(del));
                }
                x -= s;
            }
            let r = Rect::from_min_size(pos2(x - s, top), Vec2::splat(s));
            ui.painter()
                .rect_filled(r.shrink(1.0), 3.0, Color32::GRAY.gamma_multiply(0.5));
            let h = ui.put(
                r,
                egui::Label::new(RichText::new(I_MOVE).size(s * 0.65).color(Color32::WHITE))
                    .sense(Sense::drag()),
            );
            if h.drag_started() {
                self.drag = Some(Drag::Node(cell.node));
            }
            h.on_hover_cursor(CursorIcon::Grab)
                .on_hover_text("Drag to move (drop outside to delete)");
        }
        // switch: add a case / the default case, delete
        let w = (1.8 * f).max(30.0);
        if let Some((id, r, has_default)) = out.switches.iter().find(|(_, r, _)| {
            r.contains(p) || Rect::from_min_size(r.right_top(), vec2(w, r.height())).contains(p)
        }) {
            let ops = Rect::from_min_size(r.right_top(), vec2(w, r.height().max(3.0 * s)));
            ui.painter().rect(
                ops,
                CornerRadius {
                    ne: 4,
                    se: 4,
                    ..Default::default()
                },
                Color32::from_rgb(0xe5, 0xf2, 0xe5),
                Stroke::new(bw.max(1.0), Color32::BLACK),
                StrokeKind::Inside,
            );
            let mut col = Vec::new();
            col.push(("+C", GREEN, "Add case", Action::AddCase(*id, false)));
            if !has_default {
                col.push(("+D", GREEN, "Add default case", Action::AddCase(*id, true)));
            }
            col.push((
                I_CLOSE,
                RED,
                "Delete the multi-way decision",
                Action::Del(Del::Node(*id)),
            ));
            let n = col.len() as f32;
            for (i, (t, c, tip, a)) in col.into_iter().enumerate() {
                let y = ops.center().y + (i as f32 - (n - 1.0) / 2.0) * s;
                let br = Rect::from_center_size(pos2(ops.center().x, y), vec2(w, s));
                let b = egui::Button::new(RichText::new(t).size(s * 0.7).color(c)).frame(false);
                if ui.put(br, b).on_hover_text(tip).clicked() {
                    actions.push(a);
                }
            }
        }
        // structogram: delete, function header, local declarations, return value
        let bh = if platform::MOBILE { 26.0 } else { 20.0 };
        for (id, r) in &out.diagrams {
            let Some(d) = self.program.diagrams.iter().find(|d| d.id == *id) else {
                continue;
            };
            let mut items = vec![(
                I_CLOSE,
                None,
                "Delete this structogram",
                Action::DelDiagram(*id),
            )];
            items.push((
                "F",
                Some(d.show_header),
                "Show / hide the function header",
                Action::Toggle(*id, 0),
            ));
            items.push((
                "V",
                Some(d.show_variables),
                "Show / hide the local declarations",
                Action::Toggle(*id, 1),
            ));
            if d.show_header {
                items.push((
                    "R",
                    Some(d.show_return),
                    "Show / hide the return value",
                    Action::Toggle(*id, 2),
                ));
            }
            items.push((
                I_IMAGE,
                Some(false),
                "Copy this structogram as a picture (right-click it for more)",
                Action::CopyImage(*id),
            ));
            let bw_item = bh + 4.0;
            let bar = Rect::from_min_size(
                r.left_top() - vec2(0.0, bh),
                vec2(items.len() as f32 * bw_item, bh),
            );
            if !(r.contains(p) || bar.contains(p)) {
                continue;
            }
            let painter = ui.painter().clone();
            let v = ui.visuals().clone();
            painter.rect(
                bar,
                CornerRadius {
                    nw: 4,
                    ne: 4,
                    ..Default::default()
                },
                v.window_fill,
                Stroke::new(1.0, v.widgets.noninteractive.fg_stroke.color),
                StrokeKind::Inside,
            );
            for (i, (t, on, tip, a)) in items.into_iter().enumerate() {
                let br =
                    Rect::from_min_size(bar.min + vec2(i as f32 * bw_item, 0.0), vec2(bw_item, bh));
                let resp = ui
                    .interact(br, Id::unique(("stuki-bar", *id, i)), Sense::click())
                    .on_hover_text(tip)
                    .on_hover_cursor(CursorIcon::PointingHand);
                if on == Some(true) {
                    painter.rect_filled(br.shrink(2.0), 3.0, v.selection.bg_fill);
                } else if resp.hovered() {
                    painter.rect_filled(br.shrink(2.0), 3.0, v.widgets.hovered.weak_bg_fill);
                }
                let color = match on {
                    _ if t == I_IMAGE => v.text_color(),
                    None => RED,
                    Some(true) => v.strong_text_color(),
                    Some(false) => v.weak_text_color(),
                };
                painter.text(
                    br.center(),
                    Align2::CENTER_CENTER,
                    t,
                    egui::FontId::proportional(bh * 0.6),
                    color,
                );
                if resp.clicked() {
                    actions.push(a);
                }
            }
        }
    }

    fn text_edit(&mut self, ui: &mut egui::Ui, tb: &layout::TextBox) {
        let id = text_id(tb.key);
        let Some(rich) = self.program.text_mut(tb.key) else {
            return;
        };
        let before = rich.clone();
        let (font, color) = (tb.font.clone(), tb.color);
        let shadow = before.clone();
        let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _w: f32| {
            // colors follow the text while it is being typed
            let mut r = shadow.clone();
            r.text = buf.as_str().to_string();
            r.edited(&shadow.text);
            ui.ctx()
                .fonts_mut(|f| f.layout_job(rich_job(&r.text, &r.styles, &font, color)))
        };
        let hint = RichText::new(tb.hint)
            .font(tb.font.clone())
            .color(color.gamma_multiply(0.45));
        let w = tb.rect.width().max(4.0) + 2.0;
        let te = TextEdit::multiline(&mut rich.text)
            .id(id)
            .frame(Frame::NONE)
            .margin(Margin::ZERO)
            .desired_rows(1)
            .desired_width(w)
            .hint_text(hint)
            .layouter(&mut layouter);
        let r = ui.put(
            Rect::from_min_size(tb.rect.min, vec2(w, tb.rect.height())),
            te,
        );
        let changed = r.changed();
        if changed {
            rich.edited(&before.text);
        }
        if changed && self.armed != Some(tb.key) {
            let mut snap = self.program.clone();
            if let Some(t) = snap.text_mut(tb.key) {
                *t = before;
            }
            self.undo.push(snap);
            self.redo.clear();
            self.armed = Some(tb.key);
        }
        if r.lost_focus() && self.armed == Some(tb.key) {
            self.armed = None;
        }
        if r.has_focus() {
            self.last_text = Some(tb.key);
        }
        if r.gained_focus() || r.clicked() {
            self.last_text = Some(tb.key);
            match tb.key {
                TextKey::Name(d) | TextKey::Vars(d) | TextKey::Ret(d) => {
                    self.focus_diagram = Some(d)
                }
                k => self.selected = self.program.owner(k),
            }
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
                        0 => &mut d.show_header,
                        1 => &mut d.show_variables,
                        _ => &mut d.show_return,
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

fn collect_ids(n: &Node, out: &mut HashSet<u64>) {
    out.insert(n.id);
    for s in n.seqs() {
        for c in s {
            collect_ids(c, out);
        }
    }
}

/// The small example of an element shown in the Elements menu.
fn preview(tag: Tag) -> Diagram {
    let mut n = Node::new(tag);
    match &mut n.kind {
        Kind::Statement(t) => *t = Rich::plain("statement"),
        Kind::If { cond, .. } | Kind::While { cond, .. } | Kind::Repeat { cond, .. } => {
            *cond = Rich::plain("cond")
        }
        Kind::Switch { cases } => {
            for (i, c) in cases.iter_mut().enumerate() {
                if let Some(t) = &mut c.cond {
                    *t = Rich::plain(&format!("cond{}", i + 1));
                }
            }
        }
        Kind::Noop => {}
    }
    Diagram {
        body: vec![n],
        ..Diagram::new()
    }
}

fn swatch(ui: &mut egui::Ui, color: Option<Color32>, name: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    let p = ui.painter();
    match color {
        Some(c) => {
            p.rect_filled(r, 2.0, c);
        }
        None => {
            p.rect_filled(r, 2.0, Color32::WHITE);
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                "∅",
                egui::FontId::proportional(13.0),
                Color32::BLACK,
            );
        }
    }
    let stroke = if resp.hovered() {
        Stroke::new(2.0, ACCENT)
    } else {
        Stroke::new(1.0, Color32::LIGHT_GRAY)
    };
    p.rect_stroke(r, 2.0, stroke, StrokeKind::Inside);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

fn help(ui: &mut egui::Ui) {
    ui.label(RichText::new("How to use").strong());
    for p in [
        "Drag elements from the Elements menu and drop them onto a structogram: above or below an element, or onto an empty statement “-” to replace it. To put it after a loop or decision, drop it on that element's bottom edge, or just below the structogram. A blue line shows where it will go. The ↑ and ↓ buttons insert the element above or below the selected element instead (click an element to select it).",
        "Move an element by dragging its empty area or the handle in its corner. Drop it outside the structograms to delete it, or click the red ✕. Hovering a multi-way decision shows +C (add case), +D (add default case) and ✕ on its right.",
        "Hovering a structogram shows a bar above it: ✕ deletes it, F shows the function header, V the local declarations, R the return value. The green + adds a new structogram.",
        "Colors: the text colors apply to the selected part of the text you clicked into last (or all of it), the element background to the selected element, the structogram background to its structogram. Edit changes a palette, which is saved with the project.",
        "Copy image (Ctrl+C) and Export PNG work on the structogram of the selected element; right-click a structogram, or use the picture button above it, to copy that one. Pictures are light (white background) unless you turn that off in the settings. Copy as text (Ctrl+Shift+C) copies the whole project as a STUKI1: line instead. Pinch or Ctrl+wheel zooms; two fingers move the editor on touch screens. Ctrl+Z / Ctrl+Y undo and redo, Delete removes the selected element.",
        "Save writes a .st file (the web app's own format). Copy puts the whole project on the clipboard as one STUKI1: line; Paste reads that, a .st file's text, or a Stuki share link (…/stuki/v1/?data=…).",
    ] {
        ui.label(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
