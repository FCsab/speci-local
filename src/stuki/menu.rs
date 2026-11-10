//! The menu above the editor: undo, pictures, appearance settings, and the Elements to drag in.

use super::layout::Lay;
use super::model::{Diagram, Kind, Node, Tag};
use super::rich::Rich;
use super::settings::{Look, Settings, ZOOM_RANGE};
use super::{Drag, Stuki};
use crate::platform;
use crate::ui::{I_BRUSH, I_DOWNLOAD, I_HELP, I_IMAGE, I_REDO, I_SLIDERS, I_TEXT, I_UNDO};
use eframe::egui::{self, CursorIcon, Id, Rect, RichText, Sense, StrokeKind, Vec2, pos2, vec2};

impl Stuki {
    pub(super) fn menu(&mut self, ui: &mut egui::Ui) {
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
                        .range(ZOOM_RANGE)
                        .speed(0.5)
                        .prefix("Zoom "),
                );
            } else {
                ui.add(
                    egui::Slider::new(&mut self.settings.zoom, ZOOM_RANGE)
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
            ui.checkbox(
                &mut s.light_pictures,
                "Light pictures (white background, also in dark mode)",
            );
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

pub(super) fn help(ui: &mut egui::Ui) {
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
