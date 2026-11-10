//! A whole structogram: the body, and optionally the function frame around it with the header,
//! the local declarations and the return value.

use super::{Lay, NO_PAD, css_color};
use crate::stuki::model::{Diagram, TextKey};
use eframe::egui::{Align2, CornerRadius, Galley, Pos2, Rect, Shape, StrokeKind, Vec2, pos2, vec2};
use std::sync::Arc;

impl Lay<'_> {
    fn small(&self, text: &str) -> Arc<Galley> {
        self.label(text, 0.4, self.look.text.gamma_multiply(0.5))
    }

    /// (variables box, body, footer) sizes and the total size of a diagram.
    fn parts(&mut self, d: &Diagram) -> (Option<Vec2>, Vec2, Option<Vec2>, Option<Vec2>, Vec2) {
        let (f, bw, vp) = (self.f, self.bw, self.vp);
        let tc = self.look.text;
        let vars = d.show_variables.then(|| {
            let t = self.text(TextKey::Vars(d.id), &d.variables, 0.8, "variables", tc);
            let l = self.small("Local declarations").size();
            vec2(t.x.max(l.x) + f + bw, l.y + t.y + 2.0 * vp + 0.25 * f)
        });
        let body = self.block_size(&d.body, NO_PAD);
        if !d.show_header {
            let w = body.x.max(vars.map_or(0.0, |v| v.x));
            return (
                vars,
                body,
                None,
                None,
                vec2(w, body.y + vars.map_or(0.0, |v| v.y)),
            );
        }
        let footer = d.show_return.then(|| {
            let t = self.text(TextKey::Ret(d.id), &d.ret, 1.0, "return value", tc);
            let l = self.small("Return value").size();
            vec2(t.x.max(l.x) + f + bw, l.y + t.y + 2.0 * vp)
        });
        let ht = self.text(TextKey::Name(d.id), &d.name, 1.0, "Header", tc);
        let hl = self.small("Function").size();
        let header = vec2(ht.x.max(hl.x) + f, hl.y + ht.y + 2.0 * vp + 0.25 * f);
        let col_w = [Some(body), vars, footer]
            .iter()
            .flatten()
            .map(|v| v.x)
            .fold(0.0, f32::max);
        let col_h: f32 = [Some(body), vars, footer]
            .iter()
            .flatten()
            .map(|v| v.y)
            .sum();
        let right = if self.s.right_padding { 1.25 * f } else { 0.0 };
        let bottom = if footer.is_some() { 0.5 * f } else { 1.25 * f };
        let w = header.x.max(1.25 * f + col_w + right) + 1.5 * bw;
        let h = header.y + col_h + bottom + 2.0 * bw;
        (vars, body, footer, Some(header), vec2(w, h))
    }

    pub fn diagram_size(&mut self, d: &Diagram) -> Vec2 {
        self.parts(d).4
    }

    pub fn place_diagram(&mut self, d: &Diagram, at: Pos2) -> Rect {
        let (vars, body, footer, header, size) = self.parts(d);
        let (f, bw, vp) = (self.f, self.bw, self.vp);
        let bg = d.bg.as_deref().and_then(css_color).unwrap_or(self.look.bg);
        let rect = Rect::from_min_size(at, size);
        let tc = self.look.text;
        let dotted = |lay: &mut Self, pts: Vec<Pos2>| {
            let c = lay.look.border.gamma_multiply(0.2);
            lay.out
                .lines
                .extend(Shape::dotted_line(&pts, c, bw * 2.0, bw / 2.0));
        };
        let place_vars = |lay: &mut Self, r: Rect| {
            dotted(
                lay,
                vec![
                    r.left_bottom(),
                    r.left_top(),
                    r.right_top(),
                    r.right_bottom(),
                ],
            );
            let l = lay.small("Local declarations");
            let lh = l.size().y;
            let inner = r.shrink2(vec2(bw / 2.0 + 0.5 * f, vp));
            lay.put_label(l, inner, Align2::LEFT_TOP);
            let size = lay.text(TextKey::Vars(d.id), &d.variables, 0.8, "variables", tc);
            let at = Rect::from_min_max(inner.min + vec2(0.0, lh), inner.max);
            lay.put_text(
                TextKey::Vars(d.id),
                size,
                at,
                Align2::LEFT_TOP,
                0.8,
                "variables",
                tc,
            );
        };
        let Some(header) = header else {
            let mut y = at.y;
            if let Some(v) = vars {
                place_vars(self, Rect::from_min_size(at, vec2(size.x, v.y)));
                y += v.y;
            }
            let body_r = Rect::from_min_max(pos2(at.x, y), rect.max);
            self.out.fills.push(Shape::rect_filled(body_r, 0.0, bg));
            let last = self.place_block(&d.body, body_r, NO_PAD, bg);
            self.out.ends.push((rect, body_r, last));
            self.out.diagrams.push((d.id, rect));
            return rect;
        };
        // function frame: rounded border, "Function" + name, then the indented body
        let frame = rect.shrink(bw / 2.0);
        self.out.fills.push(Shape::rect_filled(
            frame,
            CornerRadius::same((0.25 * f) as u8),
            bg,
        ));
        self.out.lines.push(Shape::rect_stroke(
            frame,
            CornerRadius::same((0.25 * f) as u8),
            self.stroke(),
            StrokeKind::Middle,
        ));
        let mut y = at.y + bw;
        let l = self.small("Function");
        let lh = l.size().y;
        self.put_label(
            l,
            Rect::from_min_size(pos2(at.x + bw + 0.5 * f, y), vec2(size.x, lh)),
            Align2::LEFT_TOP,
        );
        let ht = self.text(TextKey::Name(d.id), &d.name, 1.0, "Header", tc);
        let hr = Rect::from_min_size(pos2(at.x + bw + 0.5 * f, y + lh + vp), vec2(size.x, ht.y));
        self.put_text(
            TextKey::Name(d.id),
            ht,
            hr,
            Align2::LEFT_TOP,
            1.0,
            "Header",
            tc,
        );
        y += header.y;
        let left = at.x + bw + 1.25 * f;
        // without right padding the body shares the frame's right border
        let right = rect.right()
            - bw / 2.0
            - if self.s.right_padding {
                1.25 * f + bw / 2.0
            } else {
                0.0
            };
        if let Some(v) = vars {
            place_vars(
                self,
                Rect::from_min_max(pos2(left, y), pos2(right, y + v.y)),
            );
            y += v.y;
        }
        let body_r = Rect::from_min_max(pos2(left, y), pos2(right, y + body.y));
        let last = self.place_block(&d.body, body_r, NO_PAD, bg);
        self.out.ends.push((rect, body_r, last));
        y += body.y;
        if let Some(fv) = footer {
            let r = Rect::from_min_max(pos2(left, y), pos2(right, y + fv.y));
            dotted(
                self,
                vec![
                    r.left_top(),
                    r.left_bottom(),
                    r.right_bottom(),
                    r.right_top(),
                ],
            );
            let inner = r.shrink2(vec2(bw / 2.0 + 0.5 * f, vp));
            let t = self.text(TextKey::Ret(d.id), &d.ret, 1.0, "return value", tc);
            self.put_text(
                TextKey::Ret(d.id),
                t,
                inner,
                Align2::LEFT_TOP,
                1.0,
                "return value",
                tc,
            );
            let l = self.small("Return value");
            self.put_label(l, inner, Align2::LEFT_BOTTOM);
        }
        self.out.diagrams.push((d.id, rect));
        rect
    }
}
