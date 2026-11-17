//! Putting the elements where they belong: shapes to draw, texts to edit, and the rectangles
//! the editor needs for clicks, hovering and drag & drop.

use super::{Cell, Del, Hit, Lay, NO_PAD, Pad, SELECTED, css_color, kind_color, multiply};
use crate::stuki::model::{Case, Kind, Node, TextKey};
use crate::stuki::rich::Rich;
use eframe::egui::{Align2, Color32, Rect, Shape, pos2, vec2};

impl Lay<'_> {
    fn fill_of(&self, n: &Node, under: Color32) -> (Option<Color32>, Color32) {
        let own = self.s.colorized.then(|| kind_color(&n.kind));
        let bg = n.bg.as_deref().and_then(css_color);
        let fill = if self.selected == Some(n.id) {
            Some(bg.map_or(SELECTED, |b| multiply(b, SELECTED)))
        } else {
            // like the web app's inline `background`, a chosen color replaces the kind's color
            match bg {
                Some(b) => Some(multiply(under, b)),
                None => own,
            }
        };
        (fill, fill.unwrap_or(under))
    }

    fn inner(&self, r: Rect, px: f32) -> Rect {
        let d = vec2(self.bw / 2.0 + px, self.bw / 2.0 + self.vp);
        Rect::from_min_max(r.min + d, r.max - d)
    }

    fn row_height(&self, text_h: f32) -> f32 {
        text_h + 2.0 * self.vp + self.bw
    }

    fn cell(&mut self, rect: Rect, del: Option<Del>, node: u64) {
        self.out.cells.push(Cell { rect, del, node });
    }

    pub(super) fn place_node(&mut self, n: &Node, r: Rect, under: Color32) {
        let hit = self.out.hits.len();
        self.out.hits.push(Hit {
            id: n.id,
            rect: r,
            grip: r,
            noop: n.is_noop(),
            parent: self.parent,
            first: false,
            last: false,
        });
        let outer = self.parent.replace(hit);
        let (fill, under) = self.fill_of(n, under);
        if let Some(c) = fill {
            self.out.fills.push(Shape::rect_filled(r, 0.0, c));
        }
        let grip = match &n.kind {
            Kind::Noop => self.place_noop(r),
            Kind::Statement(t) => self.place_statement(n, t, r),
            Kind::While { cond, body } => self.place_loop(n, cond, body, true, r, under),
            Kind::Repeat { cond, body } => self.place_loop(n, cond, body, false, r, under),
            Kind::If { cond, yes, no } => self.place_if(n, cond, yes, no, r, under),
            Kind::Switch { cases } => self.place_switch(n, cases, r, under),
        };
        self.out.hits[hit].grip = grip;
        self.parent = outer;
    }

    fn place_noop(&mut self, r: Rect) -> Rect {
        self.outline(r);
        let g = self.label("-", 1.0, self.cell_text());
        self.put_label(g, self.inner(r, 0.5 * self.f), Align2::CENTER_TOP);
        r
    }

    fn place_statement(&mut self, n: &Node, text: &Rich, r: Rect) -> Rect {
        let (key, tc) = (TextKey::Node(n.id), self.cell_text());
        self.outline(r);
        let size = self.text(key, text, 1.0, "statement", tc);
        let at = self.inner(r, 0.5 * self.f);
        self.put_text(key, size, at, Align2::LEFT_TOP, 1.0, "statement", tc);
        self.cell(r, Some(Del::Node(n.id)), n.id);
        r
    }

    fn place_loop(
        &mut self,
        n: &Node,
        cond: &Rich,
        body: &[Node],
        pre: bool,
        r: Rect,
        under: Color32,
    ) -> Rect {
        let (key, tc) = (TextKey::Node(n.id), self.cell_text());
        self.outline(r);
        let size = self.text(key, cond, 1.0, "condition", tc);
        let ch = self.row_height(size.y);
        let indent = r.left() + 1.25 * self.f;
        let (cond_r, body_r) = if pre {
            (
                Rect::from_min_size(r.min, vec2(r.width(), ch)),
                Rect::from_min_max(pos2(indent, r.top() + ch), r.max),
            )
        } else {
            (
                Rect::from_min_max(pos2(r.left(), r.bottom() - ch), r.max),
                Rect::from_min_max(pos2(indent, r.top()), pos2(r.right(), r.bottom() - ch)),
            )
        };
        let at = self.inner(cond_r, 0.5 * self.f);
        self.put_text(key, size, at, Align2::LEFT_CENTER, 1.0, "condition", tc);
        self.cell(cond_r, Some(Del::Node(n.id)), n.id);
        self.place_block(body, body_r, NO_PAD, under);
        cond_r
    }

    fn place_if(
        &mut self,
        n: &Node,
        cond: &Rich,
        yes: &[Node],
        no: &[Node],
        r: Rect,
        under: Color32,
    ) -> Rect {
        let (f, bw) = (self.f, self.bw);
        let (key, tc) = (TextKey::Node(n.id), self.cell_text());
        let size = self.text(key, cond, 1.0, "condition", tc);
        let cond_r = Rect::from_min_size(r.min, vec2(r.width(), self.row_height(size.y)));
        self.outline(cond_r);
        let at = self.inner(cond_r, f);
        self.put_text(key, size, at, Align2::CENTER_CENTER, 1.0, "condition", tc);
        // the T and F corners
        let d = 1.25 * f;
        self.line(cond_r.left_top(), pos2(cond_r.left() + d, cond_r.bottom()));
        self.line(
            cond_r.right_top(),
            pos2(cond_r.right() - d, cond_r.bottom()),
        );
        let half = tc.gamma_multiply(0.5);
        let corner = Rect::from_min_max(
            pos2(cond_r.left() + bw / 2.0 + 0.175 * f, cond_r.top()),
            pos2(
                cond_r.right() - bw / 2.0 - 0.175 * f,
                cond_r.bottom() - bw / 2.0,
            ),
        );
        let t = self.label("T", 0.7, half);
        self.put_label(t, corner, Align2::LEFT_BOTTOM);
        let fl = self.label("F", 0.7, half);
        self.put_label(fl, corner, Align2::RIGHT_BOTTOM);
        self.cell(cond_r, Some(Del::Node(n.id)), n.id);
        // branches share the extra width equally
        let (l, rr) = (
            self.block_size(yes, self.branch).x,
            self.block_size(no, self.branch).x,
        );
        let extra = (r.width() - l - rr).max(0.0) / 2.0;
        let mid = r.left() + l + extra;
        self.place_block(
            yes,
            Rect::from_min_max(pos2(r.left(), cond_r.bottom()), pos2(mid, r.bottom())),
            self.branch,
            under,
        );
        self.place_block(
            no,
            Rect::from_min_max(pos2(mid, cond_r.bottom()), r.max),
            self.branch,
            under,
        );
        cond_r
    }

    fn place_switch(&mut self, n: &Node, cases: &[Case], r: Rect, under: Color32) -> Rect {
        let (f, bw, vp) = (self.f, self.bw, self.vp);
        let tc = self.cell_text();
        let (cols, ch) = self.switch_cols(n);
        let extra = (r.width() - cols.iter().sum::<f32>()).max(0.0) / cols.len().max(1) as f32;
        let mut x = r.left();
        let deletable = cases.iter().filter(|c| c.cond.is_some()).count() > 2;
        let d = 1.25 * f;
        for (c, w) in cases.iter().zip(cols) {
            let w = w + extra;
            let cond_r = Rect::from_min_max(pos2(x, r.top()), pos2(x + w, r.top() + ch));
            self.outline(cond_r);
            let case = Del::Case {
                switch: n.id,
                case: c.id,
            };
            let del = match &c.cond {
                Some(t) => {
                    self.line(cond_r.left_top(), pos2(cond_r.left() + d, cond_r.bottom()));
                    let key = TextKey::Case(c.id);
                    let size = self.text(key, t, 1.0, "condition", tc);
                    let at = Rect::from_min_max(
                        cond_r.min + vec2(bw / 2.0 + 1.25 * f, bw / 2.0 + vp),
                        cond_r.max - vec2(bw / 2.0 + f, bw / 2.0 + vp),
                    );
                    self.put_text(key, size, at, Align2::CENTER_CENTER, 1.0, "condition", tc);
                    deletable.then_some(case)
                }
                None => {
                    self.line(
                        cond_r.right_top(),
                        pos2(cond_r.right() - d, cond_r.bottom()),
                    );
                    let g = self.label("default", 0.7, tc.gamma_multiply(0.5));
                    let at = Rect::from_min_max(
                        cond_r.min,
                        cond_r.max - vec2(bw / 2.0 + 0.175 * f, bw / 2.0),
                    );
                    self.put_label(g, at, Align2::RIGHT_BOTTOM);
                    Some(case)
                }
            };
            self.cell(cond_r, del, n.id);
            let body_r = Rect::from_min_max(pos2(x, cond_r.bottom()), pos2(x + w, r.bottom()));
            self.place_block(&c.body, body_r, self.branch, under);
            x += w;
        }
        let has_default = cases.last().is_some_and(|c| c.cond.is_none());
        self.out.switches.push((n.id, r, has_default));
        Rect::from_min_size(r.min, vec2(r.width(), ch))
    }

    /// Returns the index of the last element's hit.
    pub(super) fn place_block(&mut self, seq: &[Node], r: Rect, pad: Pad, under: Color32) -> usize {
        self.outline(r);
        let sp = self.sp;
        let inner = Rect::from_min_max(
            r.min + vec2(pad.l + sp, pad.t + sp),
            r.max - vec2(pad.r + sp, pad.b + sp),
        );
        let mut y = inner.top();
        let mut last = 0;
        for (i, n) in seq.iter().enumerate() {
            let h = if i + 1 == seq.len() {
                (inner.bottom() - y).max(0.0) // the last one takes up the rest
            } else {
                self.node_size(n).y
            };
            last = self.out.hits.len();
            self.place_node(
                n,
                Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), h)),
                under,
            );
            self.out.hits[last].first = i == 0;
            self.out.hits[last].last = i + 1 == seq.len();
            y += h + 2.0 * sp;
        }
        last
    }
}
