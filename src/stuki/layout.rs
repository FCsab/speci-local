//! Measuring and placing structograms, following the web app's CSS: the same paddings, the
//! same distances (in em), the T / F corner diagonals, function headers and so on.
//!
//! Every box is measured from line centre to line centre, so neighbouring boxes share their
//! border line. The result ([`Out`]) is a list of shapes plus the editable texts and the
//! rectangles the editor needs for clicks, hovering and drag & drop.

use super::model::{Diagram, Kind, Node, TextKey};
use super::rich::{Rich, Style};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Shape, Stroke,
    StrokeKind, Vec2, pos2, text::LayoutJob, vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// The web app's "Appearance" and "Advanced settings".
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
    /// Font size of the diagrams in points ("Zoom", 16–64).
    pub zoom: f32,
    /// Border width in em (0.1–0.2).
    pub border: f32,
    /// Horizontal / vertical distance inside branches, in 0.1 em.
    pub hdist: u32,
    pub vdist: u32,
    /// Change the two distances together.
    pub linked: bool,
    /// No distance between a condition and the branch below it.
    pub snapping: bool,
    /// Extra height of the blocks, in 0.025 em.
    pub block_height: u32,
    /// Space around the elements, in 0.1 em.
    pub spacing: u32,
    /// Space right of a function's body.
    pub right_padding: bool,
    /// Color the elements by kind.
    pub colorized: bool,
    pub proportional: bool,
    /// Copied / exported pictures always use the light look (white background, black lines),
    /// whatever the app's theme.
    pub light_pictures: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            zoom: 16.0,
            border: 0.125,
            hdist: 0,
            vdist: 0,
            linked: true,
            snapping: false,
            block_height: 0,
            spacing: 0,
            right_padding: false,
            colorized: true,
            proportional: false,
            light_pictures: true,
        }
    }
}

/// Colors that follow the light / dark theme (the web app's `--struct-*` variables).
#[derive(Clone, Copy)]
pub struct Look {
    pub border: Color32,
    pub text: Color32,
    pub bg: Color32,
}

impl Look {
    pub fn new(dark: bool) -> Look {
        if dark {
            // rgba(255,255,255,0.7) over #555, made opaque so shared lines don't get brighter
            let c = Color32::from_gray(0xcc);
            Look {
                border: c,
                text: c,
                bg: Color32::from_gray(0x55),
            }
        } else {
            Look {
                border: Color32::BLACK,
                text: Color32::BLACK,
                bg: Color32::WHITE,
            }
        }
    }
}

pub const SELECTED: Color32 = Color32::from_rgb(250, 205, 189); // hsl(16, 86%, 86%)
pub const HOVER: Color32 = Color32::from_rgb(0xe3, 0x2f, 0xe3);

fn kind_color(k: &Kind) -> Color32 {
    let hex = match k {
        Kind::Noop => 0xeeeeee,
        Kind::Statement(_) => 0xbedef3,
        Kind::If { .. } => 0xfad7a0,
        Kind::While { .. } => 0xbcefd1,
        Kind::Repeat { .. } => 0xdfcae8,
        Kind::Switch { .. } => 0xfbecb2,
    };
    rgb(hex)
}

pub fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub fn css_color(s: &str) -> Option<Color32> {
    super::rich::parse_color(s).map(rgb)
}

fn multiply(a: Color32, b: Color32) -> Color32 {
    let m = |x: u8, y: u8| ((x as u16 * y as u16 + 127) / 255) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// A galley for rich text (no wrapping).
pub fn rich_job(text: &str, styles: &[Style], font: &FontId, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let fmt = |st: Style| egui::TextFormat {
        font_id: font.clone(),
        color: st.color.map(rgb).unwrap_or(color),
        background: st.bg.map(rgb).unwrap_or(Color32::TRANSPARENT),
        ..Default::default()
    };
    if text.is_empty() {
        job.append("", 0.0, fmt(Style::default()));
        return job;
    }
    let mut start = 0;
    let mut cur: Option<Style> = None;
    for (i, (b, _)) in text.char_indices().enumerate() {
        let st = styles.get(i).copied().unwrap_or_default();
        if cur != Some(st) {
            if let Some(c) = cur {
                job.append(&text[start..b], 0.0, fmt(c));
            }
            start = b;
            cur = Some(st);
        }
    }
    job.append(&text[start..], 0.0, fmt(cur.unwrap_or_default()));
    job
}

/// One editable text and where it goes.
pub struct TextBox {
    pub key: TextKey,
    pub rect: Rect,
    pub galley: Arc<Galley>,
    pub font: FontId,
    pub color: Color32,
    pub hint: &'static str,
}

/// What the ✕ in a cell's corner deletes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Del {
    Node(u64),
    Case { switch: u64, case: u64 },
}

pub struct Cell {
    /// The box the ✕ and the drag handle sit in.
    pub rect: Rect,
    pub del: Option<Del>,
    /// The element dragged by the handle.
    pub node: u64,
}

pub struct Hit {
    pub id: u64,
    pub rect: Rect,
    /// The element's own part (its condition row for loops and decisions): dropping on its
    /// upper half puts things before the element, on its lower half after it.
    pub grip: Rect,
    pub noop: bool,
    /// The loop / decision whose block holds this element (index into [`Out::hits`]).
    pub parent: Option<usize>,
    /// First / last element of its block.
    pub first: bool,
    pub last: bool,
}

#[derive(Default)]
pub struct Out {
    pub fills: Vec<Shape>,
    pub lines: Vec<Shape>,
    pub labels: Vec<Shape>,
    pub texts: Vec<TextBox>,
    /// Every element, outer ones first.
    pub hits: Vec<Hit>,
    pub cells: Vec<Cell>,
    /// Switches: (id, rect, has a default case).
    pub switches: Vec<(u64, Rect, bool)>,
    /// Diagrams: (id, rect).
    pub diagrams: Vec<(u64, Rect)>,
    /// Per diagram: its rect, the rect of its body, and the last element of the body
    /// (index into `hits`), for drops below a structogram.
    pub ends: Vec<(Rect, Rect, usize)>,
}

impl Out {
    /// Everything as shapes; `texts` too when there is no text editor on top (PNG, previews).
    pub fn shapes(&self, with_text: bool) -> Vec<Shape> {
        let mut v = self.fills.clone();
        v.extend(self.lines.iter().cloned());
        v.extend(self.labels.iter().cloned());
        if with_text {
            for t in &self.texts {
                v.push(Shape::galley(t.rect.min, t.galley.clone(), t.color));
            }
        }
        v
    }
}

#[derive(Clone, Copy)]
struct Pad {
    l: f32,
    t: f32,
    r: f32,
    b: f32,
}

const NO_PAD: Pad = Pad {
    l: 0.0,
    t: 0.0,
    r: 0.0,
    b: 0.0,
};

pub struct Lay<'a> {
    ctx: &'a egui::Context,
    s: &'a Settings,
    look: Look,
    pub selected: Option<u64>,
    /// Draw placeholders like "statement" in empty texts (only the PNG leaves them out).
    pub hints: bool,
    /// Font size, border width, block height, branch padding, spacing, in points.
    f: f32,
    bw: f32,
    vp: f32,
    branch: Pad,
    sp: f32,
    family: FontFamily,
    galleys: HashMap<TextKey, Arc<Galley>>,
    /// While placing a block's elements: the hit of the element owning the block.
    parent: Option<usize>,
    sizes: HashMap<u64, Vec2>,
    pub out: Out,
}

impl<'a> Lay<'a> {
    /// `scale` multiplies every size (the PNG is drawn larger than the screen).
    pub fn new(ctx: &'a egui::Context, s: &'a Settings, look: Look, scale: f32) -> Self {
        let f = s.zoom * scale;
        let (hd, vd) = (s.hdist as f32 * 0.1 * f, s.vdist as f32 * 0.1 * f);
        Lay {
            ctx,
            s,
            look,
            selected: None,
            hints: true,
            f,
            bw: s.border * f,
            vp: s.block_height as f32 * 0.025 * f,
            branch: Pad {
                l: hd,
                t: if s.snapping { 0.0 } else { vd },
                r: hd,
                b: vd,
            },
            sp: s.spacing as f32 * 0.1 * f,
            family: if s.proportional {
                FontFamily::Proportional
            } else {
                FontFamily::Monospace
            },
            galleys: HashMap::new(),
            parent: None,
            sizes: HashMap::new(),
            out: Out::default(),
        }
    }

    pub fn em(&self) -> f32 {
        self.f
    }

    pub fn font(&self, em: f32) -> FontId {
        FontId::new(self.f * em, self.family.clone())
    }

    fn cell_text(&self) -> Color32 {
        if self.s.colorized {
            Color32::BLACK
        } else {
            self.look.text
        }
    }

    fn label(&self, text: &str, em: f32, color: Color32) -> Arc<Galley> {
        let font = self.font(em);
        self.ctx
            .fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, color))
    }

    fn put_label(&mut self, g: Arc<Galley>, rect: Rect, align: Align2) {
        let pos = align.align_size_within_rect(g.size(), rect).min;
        self.out
            .labels
            .push(Shape::galley(pos, g, Color32::PLACEHOLDER));
    }

    /// Size of an editable text (its placeholder's size while empty).
    fn text(
        &mut self,
        key: TextKey,
        r: &Rich,
        em: f32,
        hint: &'static str,
        color: Color32,
    ) -> Vec2 {
        let font = self.font(em);
        let g = self
            .ctx
            .fonts_mut(|f| f.layout_job(rich_job(&r.text, &r.styles, &font, color)));
        let mut size = g.size();
        if r.text.is_empty() {
            let h = self.label(hint, em, color);
            size = size.max(h.size());
        }
        size.y = size.y.max(self.ctx.fonts_mut(|f| f.row_height(&font)));
        self.galleys.insert(key, g);
        size
    }

    #[allow(clippy::too_many_arguments)]
    fn put_text(
        &mut self,
        key: TextKey,
        size: Vec2,
        at: Rect,
        align: Align2,
        em: f32,
        hint: &'static str,
        color: Color32,
    ) {
        let Some(g) = self.galleys.get(&key).cloned() else {
            return;
        };
        let rect = align.align_size_within_rect(size, at);
        if g.text().is_empty() && !self.hints {
            return;
        }
        self.out.texts.push(TextBox {
            key,
            rect,
            galley: g,
            font: self.font(em),
            color,
            hint,
        });
    }

    fn stroke(&self) -> Stroke {
        Stroke::new(self.bw, self.look.border)
    }

    fn outline(&mut self, r: Rect) {
        self.out.lines.push(Shape::rect_stroke(
            r,
            0.0,
            self.stroke(),
            StrokeKind::Middle,
        ));
    }

    fn line(&mut self, a: Pos2, b: Pos2) {
        self.out
            .lines
            .push(Shape::line_segment([a, b], self.stroke()));
    }

    // ------------------------------------------------------------ measuring

    fn node_size(&mut self, n: &Node) -> Vec2 {
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

    fn block_size(&mut self, seq: &[Node], pad: Pad) -> Vec2 {
        let (mut w, mut h) = (0.0f32, 0.0);
        for n in seq {
            let s = self.node_size(n);
            w = w.max(s.x);
            h += s.y + 2.0 * self.sp;
        }
        vec2(w + 2.0 * self.sp + pad.l + pad.r, h + pad.t + pad.b)
    }

    /// Switch columns: width of each case's column.
    fn switch_cols(&mut self, n: &Node) -> (Vec<f32>, f32) {
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

    // ------------------------------------------------------------ placing

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

    fn place_node(&mut self, n: &Node, r: Rect, under: Color32) {
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
        let (f, bw, vp) = (self.f, self.bw, self.vp);
        let tc = self.cell_text();
        let inner = |r: Rect, px: f32| {
            Rect::from_min_max(
                r.min + vec2(bw / 2.0 + px, bw / 2.0 + vp),
                r.max - vec2(bw / 2.0 + px, bw / 2.0 + vp),
            )
        };
        match &n.kind {
            Kind::Noop => {
                self.outline(r);
                let g = self.label("-", 1.0, tc);
                self.put_label(g, inner(r, 0.5 * f), Align2::CENTER_TOP);
                self.out.cells.push(Cell {
                    rect: r,
                    del: None,
                    node: n.id,
                });
            }
            Kind::Statement(t) => {
                self.outline(r);
                let size = self.text(TextKey::Node(n.id), t, 1.0, "statement", tc);
                self.put_text(
                    TextKey::Node(n.id),
                    size,
                    inner(r, 0.5 * f),
                    Align2::LEFT_TOP,
                    1.0,
                    "statement",
                    tc,
                );
                self.out.cells.push(Cell {
                    rect: r,
                    del: Some(Del::Node(n.id)),
                    node: n.id,
                });
            }
            Kind::While { cond, body } | Kind::Repeat { cond, body } => {
                let pre = matches!(n.kind, Kind::While { .. });
                self.outline(r);
                let size = self.text(TextKey::Node(n.id), cond, 1.0, "condition", tc);
                let ch = size.y + 2.0 * vp + bw;
                let (cond_r, body_r) = if pre {
                    (
                        Rect::from_min_size(r.min, vec2(r.width(), ch)),
                        Rect::from_min_max(pos2(r.left() + 1.25 * f, r.top() + ch), r.max),
                    )
                } else {
                    (
                        Rect::from_min_max(pos2(r.left(), r.bottom() - ch), r.max),
                        Rect::from_min_max(
                            pos2(r.left() + 1.25 * f, r.top()),
                            pos2(r.right(), r.bottom() - ch),
                        ),
                    )
                };
                self.put_text(
                    TextKey::Node(n.id),
                    size,
                    inner(cond_r, 0.5 * f),
                    Align2::LEFT_CENTER,
                    1.0,
                    "condition",
                    tc,
                );
                self.out.cells.push(Cell {
                    rect: cond_r,
                    del: Some(Del::Node(n.id)),
                    node: n.id,
                });
                self.out.hits[hit].grip = cond_r;
                self.place_block(body, body_r, NO_PAD, under);
            }
            Kind::If { cond, yes, no } => {
                let size = self.text(TextKey::Node(n.id), cond, 1.0, "condition", tc);
                let ch = size.y + 2.0 * vp + bw;
                let cond_r = Rect::from_min_size(r.min, vec2(r.width(), ch));
                self.outline(cond_r);
                self.put_text(
                    TextKey::Node(n.id),
                    size,
                    inner(cond_r, f),
                    Align2::CENTER_CENTER,
                    1.0,
                    "condition",
                    tc,
                );
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
                self.out.cells.push(Cell {
                    rect: cond_r,
                    del: Some(Del::Node(n.id)),
                    node: n.id,
                });
                self.out.hits[hit].grip = cond_r;
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
            }
            Kind::Switch { cases } => {
                let (cols, ch) = self.switch_cols(n);
                self.out.hits[hit].grip = Rect::from_min_size(r.min, vec2(r.width(), ch));
                let extra =
                    (r.width() - cols.iter().sum::<f32>()).max(0.0) / cols.len().max(1) as f32;
                let mut x = r.left();
                let deletable = cases.iter().filter(|c| c.cond.is_some()).count() > 2;
                let half = tc.gamma_multiply(0.5);
                for (c, w) in cases.iter().zip(cols) {
                    let w = w + extra;
                    let cond_r = Rect::from_min_max(pos2(x, r.top()), pos2(x + w, r.top() + ch));
                    self.outline(cond_r);
                    let d = 1.25 * f;
                    let del = match &c.cond {
                        Some(t) => {
                            self.line(cond_r.left_top(), pos2(cond_r.left() + d, cond_r.bottom()));
                            let size = self.text(TextKey::Case(c.id), t, 1.0, "condition", tc);
                            let at = Rect::from_min_max(
                                cond_r.min + vec2(bw / 2.0 + 1.25 * f, bw / 2.0 + vp),
                                cond_r.max - vec2(bw / 2.0 + f, bw / 2.0 + vp),
                            );
                            self.put_text(
                                TextKey::Case(c.id),
                                size,
                                at,
                                Align2::CENTER_CENTER,
                                1.0,
                                "condition",
                                tc,
                            );
                            deletable.then_some(Del::Case {
                                switch: n.id,
                                case: c.id,
                            })
                        }
                        None => {
                            self.line(
                                cond_r.right_top(),
                                pos2(cond_r.right() - d, cond_r.bottom()),
                            );
                            let g = self.label("default", 0.7, half);
                            let at = Rect::from_min_max(
                                cond_r.min,
                                cond_r.max - vec2(bw / 2.0 + 0.175 * f, bw / 2.0),
                            );
                            self.put_label(g, at, Align2::RIGHT_BOTTOM);
                            Some(Del::Case {
                                switch: n.id,
                                case: c.id,
                            })
                        }
                    };
                    self.out.cells.push(Cell {
                        rect: cond_r,
                        del,
                        node: n.id,
                    });
                    let body_r =
                        Rect::from_min_max(pos2(x, cond_r.bottom()), pos2(x + w, r.bottom()));
                    self.place_block(&c.body, body_r, self.branch, under);
                    x += w;
                }
                let has_default = cases.last().is_some_and(|c| c.cond.is_none());
                self.out.switches.push((n.id, r, has_default));
            }
        }
        self.parent = outer;
    }

    /// Returns the index of the last element's hit.
    fn place_block(&mut self, seq: &[Node], r: Rect, pad: Pad, under: Color32) -> usize {
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

    // ------------------------------------------------------------ diagrams

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
