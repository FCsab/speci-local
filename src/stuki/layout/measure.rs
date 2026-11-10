//! How much room the elements need.

use super::{Lay, NO_PAD, Pad};
use crate::stuki::model::{Kind, Node, TextKey};
use eframe::egui::{Vec2, vec2};

impl Lay<'_> {
    pub(super) fn node_size(&mut self, n: &Node) -> Vec2 {
        if let Some(s) = self.sizes.get(&n.id) {
            return *s;
        }
        let (f, bw, vp) = (self.f, self.bw, self.vp);
        let tc = self.cell_text();
        let size = match &n.kind {
            Kind::Noop => {
                let t = self.label("-", 1.0, tc).size();
                vec2(t.x + f + bw, t.y + 2.0 * vp + bw)
            }
            Kind::Statement(r) => {
                let t = self.text(TextKey::Node(n.id), r, 1.0, "statement", tc);
                vec2(t.x + f + bw, t.y + 2.0 * vp + bw)
            }
            Kind::While { cond, body } | Kind::Repeat { cond, body } => {
                let t = self.text(TextKey::Node(n.id), cond, 1.0, "condition", tc);
                let b = self.block_size(body, NO_PAD);
                vec2(
                    (t.x + f + bw).max(1.25 * f + b.x),
                    t.y + 2.0 * vp + bw + b.y,
                )
            }
            Kind::If { cond, yes, no } => {
                let t = self.text(TextKey::Node(n.id), cond, 1.0, "condition", tc);
                let (l, r) = (
                    self.block_size(yes, self.branch),
                    self.block_size(no, self.branch),
                );
                vec2(
                    (t.x + 2.0 * f + bw).max(l.x + r.x),
                    t.y + 2.0 * vp + bw + l.y.max(r.y),
                )
            }
            Kind::Switch { cases } => {
                let (cols, ch) = self.switch_cols(n);
                let bh = cases
                    .iter()
                    .map(|c| self.block_size(&c.body, self.branch).y)
                    .fold(0.0, f32::max);
                vec2(cols.iter().sum(), ch + bh)
            }
        };
        self.sizes.insert(n.id, size);
        size
    }

    pub(super) fn block_size(&mut self, seq: &[Node], pad: Pad) -> Vec2 {
        let (mut w, mut h) = (0.0f32, 0.0);
        for n in seq {
            let s = self.node_size(n);
            w = w.max(s.x);
            h += s.y + 2.0 * self.sp;
        }
        vec2(w + 2.0 * self.sp + pad.l + pad.r, h + pad.t + pad.b)
    }

    /// Switch columns: width of each case's column.
    pub(super) fn switch_cols(&mut self, n: &Node) -> (Vec<f32>, f32) {
        let Kind::Switch { cases } = &n.kind else {
            return (vec![], 0.0);
        };
        let tc = self.cell_text();
        let mut cols = Vec::new();
        let mut th = 0.0f32;
        for c in cases {
            let cw = match &c.cond {
                Some(r) => {
                    let t = self.text(TextKey::Case(c.id), r, 1.0, "condition", tc);
                    th = th.max(t.y);
                    t.x + 2.25 * self.f + self.bw
                }
                None => {
                    let t = self.label("default", 0.7, tc).size();
                    th = th.max(self.label(" ", 1.0, tc).size().y);
                    (t.x + 0.25 * self.f).max(1.25 * self.f) + self.f * 0.5 + self.bw
                }
            };
            cols.push(cw.max(self.block_size(&c.body, self.branch).x));
        }
        (cols, th + 2.0 * self.vp + self.bw)
    }
}
