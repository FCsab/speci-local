//! The color palettes: text color, text background, element and structogram background.

use super::layout::css_color;
use super::rich::{hex, parse_color};
use super::{Pal, Stuki, text_id};
use crate::ui::{ACCENT, I_CLOSE, RED};
use eframe::egui::{self, Align2, Color32, CursorIcon, RichText, Sense, Stroke, StrokeKind, Vec2};

impl Stuki {
    pub(super) fn colors_panel(&mut self, ui: &mut egui::Ui) {
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
                if swatch(ui, None, &format!("{label}: none"))
                    .on_hover_text("No color")
                    .clicked()
                {
                    chosen = Some(None);
                }
                for c in &colors {
                    if swatch(ui, css_color(c), &format!("{label}: {c}"))
                        .on_hover_text(c)
                        .clicked()
                    {
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
