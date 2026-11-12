//! The editing area: the structograms themselves, their texts, hover buttons and drag & drop.

use super::layout::{self, Del, Lay, rich_job};
use super::model::{Diagram, Kind, Node, Place, TextKey};
use super::settings::{Look, ZOOM_RANGE};
use super::{Action, Drag, GREEN, Part, Stuki, Target, text_id};
use crate::platform;
use crate::ui::{ACCENT, I_CLOSE, I_DOWNLOAD, I_IMAGE, I_MOVE, I_TEXT, RED};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, Frame, Id, Margin, Pos2, Rect, RichText,
    Sense, Stroke, StrokeKind, TextEdit, Vec2, pos2, vec2,
};
use std::collections::HashSet;

impl Stuki {
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui, look: Look) {
        let ctx = ui.ctx().clone();
        if ui.ui_contains_pointer() {
            // pinch or Ctrl+wheel
            let z = ui.input(|i| i.zoom_delta());
            if z != 1.0 {
                self.settings.zoom =
                    (self.settings.zoom * z).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
            }
        }
        let mut actions = Vec::new();
        egui::ScrollArea::both()
            .id_salt("stuki-canvas")
            .auto_shrink(false)
            .show(ui, |ui| self.diagrams(ui, look, &mut actions));
        if let Some(drag) = self.drag {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
            self.drag_ghost(&ctx, drag);
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

    fn diagrams(&mut self, ui: &mut egui::Ui, look: Look, actions: &mut Vec<Action>) {
        if let Some(t) = ui.input(|i| i.multi_touch()) {
            // two fingers move the editor (one finger drags elements)
            ui.scroll_with_delta(t.translation_delta);
            if matches!(self.drag, Some(Drag::Node(_))) {
                self.drag = None;
            }
        }
        let ctx = ui.ctx().clone();
        let viewport = ui.clip_rect();
        let settings = self.settings.clone();
        let mut lay = Lay::new(&ctx, &settings, look, 1.0);
        lay.selected = self.selected;
        let f = lay.em();
        let bw = settings.border * f;
        let plus = Vec2::splat(2.2 * f);
        let sizes: Vec<Vec2> = self
            .program
            .diagrams
            .iter()
            .map(|d| lay.diagram_size(d))
            .chain([plus])
            .collect();
        let (pos, used) = flow(&sizes, ui.available_width(), f);
        let (rect, resp) =
            ui.allocate_exact_size(used.max(ui.available_size()), Sense::click_and_drag());
        for (d, p) in self.program.diagrams.iter().zip(&pos) {
            lay.place_diagram(d, rect.min + *p);
        }
        let mut out = std::mem::take(&mut lay.out);
        #[cfg(test)]
        {
            self.seen.0 = out.hits.iter().map(|h| (h.id, h.rect)).collect();
        }
        ui.painter().extend(out.shapes(false));

        let pointer = ui
            .input(|i| i.pointer.latest_pos())
            .filter(|p| viewport.contains(*p));
        let over = pointer.filter(|_| ui.ui_contains_pointer() || self.drag.is_some());
        if self.drag.is_none()
            && let Some(h) = over.and_then(|p| out.hit_at(p))
        {
            ui.painter().rect_stroke(
                h.rect,
                0.0,
                Stroke::new(bw.max(1.5), layout::HOVER),
                StrokeKind::Middle,
            );
        }

        // the "+" tile
        let plus_r = Rect::from_min_size(rect.min + pos[pos.len() - 1], plus);
        let add = ui.interact(plus_r, Id::unique("stuki-add-diagram"), Sense::click());
        let fill = if add.hovered() { 0.22 } else { 0.12 };
        let p = ui.painter();
        p.circle(
            plus_r.center(),
            (plus_r.width() - bw) / 2.0,
            GREEN.gamma_multiply(fill),
            Stroke::new(bw, GREEN.gamma_multiply(0.6)),
        );
        p.text(
            plus_r.center(),
            Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(1.6 * f),
            GREEN,
        );
        if add
            .on_hover_cursor(CursorIcon::PointingHand)
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
            self.hover_buttons(ui, &out, p, f, bw, actions);
        }

        if resp.clicked() {
            self.selected = pointer.and_then(|p| out.hit_at(p)).map(|h| h.id);
            if let Some(p) = pointer
                && let Some((d, _)) = out.diagrams.iter().find(|(_, r)| r.contains(p))
            {
                self.focus_diagram = Some(*d);
            }
        }
        // right-click a structogram: copy / export that one
        if resp.secondary_clicked() {
            self.menu_diagram =
                pointer.and_then(|p| out.diagrams.iter().position(|(_, r)| r.contains(p)));
        }
        resp.context_menu(|ui| self.context_menu(ui));
        if resp.drag_started()
            && self.drag.is_none()
            && let Some(o) = ui.input(|i| i.pointer.press_origin())
            && let Some(h) = out.hit_at(o)
        {
            self.drag = Some(Drag::Node(h.id));
        }
        if let Some(drag) = self.drag {
            let target = self.drop_target(&out, drag, pointer);
            drop_feedback(ui, &out, drag, target, bw);
            let released = ui.input(|i| i.pointer.any_released() || !i.pointer.any_down());
            if released {
                actions.push(Action::Drop(drag, target));
                self.drag = None;
            }
        }
    }

    fn context_menu(&mut self, ui: &mut egui::Ui) {
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
        if ui
            .button(format!("{I_TEXT}  Copy project as text"))
            .clicked()
        {
            self.copy_project(ui.ctx());
            ui.close();
        }
    }

    fn drag_ghost(&self, ctx: &egui::Context, drag: Drag) {
        let Some(p) = ctx.pointer_latest_pos() else {
            return;
        };
        let what = match drag {
            Drag::New(t) => t.description().to_string(),
            Drag::Node(id) => match self.program.find(id).map(|n| &n.kind) {
                Some(Kind::Statement(t)) if !t.text.is_empty() => t.text.clone(),
                Some(Kind::Noop) => "-".into(),
                Some(kind) => kind.tag().description().into(),
                None => String::new(),
            },
        };
        egui::Area::new(Id::unique("stuki-ghost"))
            .fixed_pos(p + vec2(14.0, 10.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                Frame::popup(ui.style()).show(ui, |ui| ui.label(what));
            });
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
        if let Some(cell) = out.cells.iter().rev().find(|c| c.rect.contains(p)) {
            self.cell_buttons(ui, cell, bw, actions);
        }
        let w = (1.8 * f).max(30.0);
        let beside = |r: &Rect| Rect::from_min_size(r.right_top(), vec2(w, r.height()));
        if let Some((id, r, has_default)) = out
            .switches
            .iter()
            .find(|(_, r, _)| r.contains(p) || beside(r).contains(p))
        {
            let ops = Rect::from_min_size(r.right_top(), vec2(w, r.height().max(3.0 * s)));
            switch_buttons(ui, *id, ops, *has_default, s, bw, actions);
        }
        for (id, r) in &out.diagrams {
            if let Some(d) = self.program.diagrams.iter().find(|d| d.id == *id) {
                diagram_bar(ui, d, *r, p, actions);
            }
        }
    }

    fn cell_buttons(
        &mut self,
        ui: &mut egui::Ui,
        cell: &layout::Cell,
        bw: f32,
        actions: &mut Vec<Action>,
    ) {
        let inner = cell.rect.shrink(bw / 2.0);
        let s = inner.height().min(inner.width() / 2.0);
        let glyph = 0.65 * s;
        let mv = Rect::from_min_size(
            pos2(inner.right() - s, inner.center().y - s / 2.0),
            Vec2::splat(s),
        );

        ui.painter()
            .rect_filled(mv, 0.0, Color32::GRAY.gamma_multiply(0.5));
        let h = ui.put(
            mv,
            egui::Label::new(RichText::new(I_MOVE).size(glyph).color(Color32::WHITE))
                .selectable(false)
                .sense(Sense::drag()),
        );
        if h.drag_started() {
            self.drag = Some(Drag::Node(cell.node));
        }
        h.on_hover_cursor(CursorIcon::Grab)
            .on_hover_text("Drag to move (drop outside to delete)");

        if let Some(del) = cell.del {
            let r = mv.translate(vec2(-s, 0.0));
            let tip = match del {
                Del::Case { .. } => "Delete this case",
                Del::Node(_) => "Delete",
            };
            let b = egui::Button::new(RichText::new(I_CLOSE).size(glyph).color(RED))
                .frame(false)
                .min_size(Vec2::splat(s));
            if ui
                .put(r, b)
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text(tip)
                .clicked()
            {
                actions.push(Action::Del(del));
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
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _w: f32| {
            // colors follow the text while it is being typed
            let job = if buf.as_str() == before.text {
                rich_job(&before.text, &before.styles, &font, color)
            } else {
                let mut r = before.clone();
                r.text = buf.as_str().to_string();
                r.edited(&before.text);
                rich_job(&r.text, &r.styles, &font, color)
            };
            ui.ctx().fonts_mut(|f| f.layout_job(job))
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
}

fn flow(sizes: &[Vec2], width: f32, em: f32) -> (Vec<Vec2>, Vec2) {
    let (gap, pad) = (1.25 * em, 0.5 * em);
    let mut pos = Vec::with_capacity(sizes.len());
    let (mut x, mut y, mut row, mut right) = (pad, pad + gap, 0.0f32, 0.0f32);
    for s in sizes {
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
    (pos, vec2(right + pad, y + row + pad + gap))
}

fn drop_feedback(ui: &egui::Ui, out: &layout::Out, drag: Drag, target: Target, bw: f32) {
    let painter = ui.painter();
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
                Stroke::new((1.5 * bw).max(3.0), ACCENT),
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
}

/// Switch: add a case / the default case, delete.
fn switch_buttons(
    ui: &mut egui::Ui,
    id: u64,
    ops: Rect,
    has_default: bool,
    s: f32,
    bw: f32,
    actions: &mut Vec<Action>,
) {
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
    let mut col = vec![("+C", GREEN, "Add case", Action::AddCase(id, false))];
    if !has_default {
        col.push(("+D", GREEN, "Add default case", Action::AddCase(id, true)));
    }
    col.push((
        I_CLOSE,
        RED,
        "Delete the multi-way decision",
        Action::Del(Del::Node(id)),
    ));
    let n = col.len() as f32;
    for (i, (t, c, tip, a)) in col.into_iter().enumerate() {
        let y = ops.center().y + (i as f32 - (n - 1.0) / 2.0) * s;
        let br = Rect::from_center_size(pos2(ops.center().x, y), vec2(ops.width(), s));
        let b = egui::Button::new(RichText::new(t).size(s * 0.7).color(c)).frame(false);
        if ui.put(br, b).on_hover_text(tip).clicked() {
            actions.push(a);
        }
    }
}

/// Structogram: delete, function header, local declarations, return value.
fn diagram_bar(ui: &mut egui::Ui, d: &Diagram, r: Rect, p: Pos2, actions: &mut Vec<Action>) {
    let id = d.id;
    let mut items = vec![
        (
            I_CLOSE,
            None,
            "Delete this structogram",
            Action::DelDiagram(id),
        ),
        (
            "F",
            Some(d.show_header),
            "Show / hide the function header",
            Action::Toggle(id, Part::Header),
        ),
        (
            "V",
            Some(d.show_variables),
            "Show / hide the local declarations",
            Action::Toggle(id, Part::Variables),
        ),
    ];
    if d.show_header {
        items.push((
            "R",
            Some(d.show_return),
            "Show / hide the return value",
            Action::Toggle(id, Part::Return),
        ));
    }
    items.push((
        I_IMAGE,
        Some(false),
        "Copy this structogram as a picture (right-click it for more)",
        Action::CopyImage(id),
    ));
    let bh = if platform::MOBILE { 26.0 } else { 20.0 };
    let bw_item = bh + 4.0;
    let bar = Rect::from_min_size(
        r.left_top() - vec2(0.0, bh),
        vec2(items.len() as f32 * bw_item, bh),
    );
    if !(r.contains(p) || bar.contains(p)) {
        return;
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
        let br = Rect::from_min_size(bar.min + vec2(i as f32 * bw_item, 0.0), vec2(bw_item, bh));
        let resp = ui
            .interact(br, Id::unique(("stuki-bar", id, i)), Sense::click())
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

fn collect_ids(n: &Node, out: &mut HashSet<u64>) {
    out.insert(n.id);
    for s in n.seqs() {
        for c in s {
            collect_ids(c, out);
        }
    }
}
