//! Measuring and placing structograms, following the web app's CSS: the same paddings, the
//! same distances (in em), the T / F corner diagonals, function headers and so on.
//!
//! Every box is measured from line centre to line centre, so neighbouring boxes share their
//! border line. The result ([`Out`]) is a list of shapes plus the editable texts and the
//! rectangles the editor needs for clicks, hovering and drag & drop.

mod diagram;
mod measure;
mod place;

use super::model::{Kind, TextKey};
use super::rich::{Rich, Style};
use super::settings::{Look, Settings};
use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Galley, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2,
    text::LayoutJob,
};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub fn shapes(&mut self, with_text: bool) -> Vec<Shape> {
        let mut v = std::mem::take(&mut self.fills);
        v.append(&mut self.lines);
        v.append(&mut self.labels);
        if with_text {
            for t in &self.texts {
                v.push(Shape::galley(t.rect.min, t.galley.clone(), t.color));
            }
        }
        v
    }

    pub fn hit_at(&self, p: Pos2) -> Option<&Hit> {
        self.hits.iter().rev().find(|h| h.rect.contains(p))
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
}
