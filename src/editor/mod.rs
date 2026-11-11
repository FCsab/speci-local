//! Code editor widget: syntax highlighting, line numbers and auto-completion.

mod complete;
mod highlight;
#[cfg(test)]
mod tests;
mod vim;

pub use complete::{Completion, Item, Kind, scan_symbols};
pub use highlight::{Palette, highlight_data, highlight_spec, palette};
pub use vim::Vim;

use eframe::egui::{
    self, FontId, Id, Key, Modifiers, RichText, TextEdit, TextFormat, text::CCursor,
    text::CCursorRange, text::LayoutJob, text_edit::TextEditOutput,
};

/// Alt+digit shortcuts for the special characters, in the order of the symbol buttons.
pub const SYMBOL_KEYS: &[(Key, &str)] = &[
    (Key::Num1, "∈"),
    (Key::Num2, "∀"),
    (Key::Num3, "∃"),
    (Key::Num4, "≠"),
    (Key::Num5, "≤"),
    (Key::Num6, "≥"),
];

#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    Spec,
    Data,
}

fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map(|(i, _)| i).unwrap_or(s.len())
}

/// Replace chars `a..b` with `insert` (honouring the `$` cursor marker) and move the cursor.
pub fn replace(ctx: &egui::Context, id: Id, text: &mut String, a: usize, b: usize, insert: &str) {
    let cursor_off = insert.find('$').map(|i| insert[..i].chars().count());
    let insert = insert.replace('$', "");
    let (ba, bb) = (char_to_byte(text, a), char_to_byte(text, b));
    text.replace_range(ba..bb, &insert);
    let pos = a + cursor_off.unwrap_or(insert.chars().count());
    set_cursor(ctx, id, pos);
}

pub fn set_cursor(ctx: &egui::Context, id: Id, pos: usize) {
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let mut cur = CCursor::default();
    cur.index.0 = pos;
    state.cursor.set_char_range(Some(CCursorRange::one(cur)));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

pub fn cursor_of(ctx: &egui::Context, id: Id) -> Option<(usize, usize)> {
    let s = egui::text_edit::TextEditState::load(ctx, id)?;
    let r = s.cursor.char_range()?.as_sorted_char_range();
    Some((r.start.0, r.end.0))
}

/// A code editor with line numbers, highlighting and completion.
/// With `vim`, the Vim keybindings are on.
#[allow(clippy::too_many_arguments)]
pub fn code_editor(
    ui: &mut egui::Ui,
    id: Id,
    text: &mut String,
    lang: Lang,
    symbols: &[Item],
    errors: &[usize],
    comp: &mut Completion,
    hint: &str,
    mut vim: Option<&mut Vim>,
) {
    let p = palette(ui.visuals().dark_mode);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let focused = ui.memory(|m| m.has_focus(id));
    if comp.owner == Some(id) && !focused {
        comp.items.clear();
    }
    // Vim Normal / Visual mode: keys are commands, not text
    let commands = |v: &Option<&mut Vim>| v.as_ref().is_some_and(|v| !v.inserting());
    let open = focused && !commands(&vim) && comp.is_open_for(id);

    // keys must be taken before the TextEdit sees them
    if open {
        popup_keys(ui, id, text, comp);
    }
    if focused && let Some(v) = vim.as_deref_mut() {
        v.handle(ui.ctx(), id, text);
    }
    let vim_cmd = commands(&vim);
    if focused && vim_cmd && comp.owner == Some(id) {
        comp.items.clear();
    }
    if focused && !vim_cmd {
        // Alt+1 … Alt+6 insert ∈ ∀ ∃ ≠ ≤ ≥ (the order of the symbol buttons)
        if lang == Lang::Spec
            && let Some(sym) = symbol_key(ui)
        {
            replace_selection(ui.ctx(), id, text, sym);
            comp.items.clear();
        }
        // Tab inserts two spaces: a real tab character is invalid in YAML test data
        if !open && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
            replace_selection(ui.ctx(), id, text, "  ");
        }
    }

    // with Vim, Escape leaves Insert mode instead of the editor
    let keep_escape = open || vim.is_some();
    let out = text_area(ui, id, text, lang, errors, hint, keep_escape, &p, &font);

    if vim_cmd && out.response.has_focus() {
        let at = vim.as_ref().and_then(|v| v.visual_cursor()).or_else(|| {
            out.cursor_range
                .filter(|r| r.primary == r.secondary)
                .map(|r| r.primary.index.0)
        });
        if let Some(at) = at {
            block_cursor(ui, &out, at, &font, p.text);
        }
    }

    // update the suggestion list after typing
    if out.response.has_focus()
        && !vim_cmd
        && let Some(r) = out.cursor_range
    {
        let changed = out.response.changed();
        comp.update(id, text, r.primary.index.0, changed, lang, symbols);
    }
    // popup under the cursor (also on the frame a click moved focus away from the editor)
    if (focused || out.response.has_focus()) && comp.is_open_for(id) {
        let cursor = out.cursor_range.map(|r| r.primary).or_else(|| {
            cursor_of(ui.ctx(), id).map(|c| CCursor {
                index: egui::text::CharIndex(c.1),
                ..Default::default()
            })
        });
        if let Some(cursor) = cursor
            && let Some(it) = popup(ui, id, comp, &out, cursor, &p, &font)
        {
            replace(ui.ctx(), id, text, comp.start, cursor.index.0, &it.insert);
            comp.items.clear();
        }
    }
}

/// Vim's block cursor over the character at `at`.
fn block_cursor(
    ui: &egui::Ui,
    out: &TextEditOutput,
    at: usize,
    font: &FontId,
    color: egui::Color32,
) {
    let cur = |i: usize| {
        let mut c = CCursor::default();
        c.index.0 = i;
        out.galley.pos_from_cursor(c)
    };
    let here = cur(at);
    let next = cur(at + 1);
    let w = if (next.top() - here.top()).abs() < 1.0 && next.left() > here.left() {
        next.left() - here.left()
    } else {
        font.size * 0.6
    };
    let rect = egui::Rect::from_min_size(here.min, egui::vec2(w, here.height()))
        .translate(out.galley_pos.to_vec2());
    ui.painter()
        .rect_filled(rect, 1.0, color.gamma_multiply(0.35));
}

fn replace_selection(ctx: &egui::Context, id: Id, text: &mut String, s: &str) {
    let n = text.chars().count();
    let (a, b) = cursor_of(ctx, id).unwrap_or((n, n));
    replace(ctx, id, text, a, b, s);
}

fn popup_keys(ui: &egui::Ui, id: Id, text: &mut String, comp: &mut Completion) {
    let mut accept: Option<Item> = None;
    ui.input_mut(|i| {
        let n = comp.items.len();
        if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
            comp.sel = (comp.sel + 1) % n;
        }
        if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
            comp.sel = (comp.sel + n - 1) % n;
        }
        if i.consume_key(Modifiers::NONE, Key::Tab) || i.consume_key(Modifiers::NONE, Key::Enter) {
            accept = comp.items.get(comp.sel).cloned();
        }
        if i.consume_key(Modifiers::NONE, Key::Escape) {
            comp.items.clear();
        }
    });
    if let Some(it) = accept {
        let cur = cursor_of(ui.ctx(), id).map_or(comp.start, |c| c.1);
        replace(ui.ctx(), id, text, comp.start, cur, &it.insert);
        comp.items.clear();
    }
}

fn symbol_key(ui: &egui::Ui) -> Option<&'static str> {
    ui.input_mut(|i| {
        let hit = SYMBOL_KEYS
            .iter()
            .find(|(k, _)| i.consume_key(Modifiers::ALT, *k))
            .map(|(_, s)| *s);
        if hit.is_some() {
            // some platforms also send the key as typed text: the digit, or on macOS
            // the Option-layer character (Option+1 is ¡)
            i.events
                .retain(|e| !matches!(e, egui::Event::Text(t) if t.chars().count() == 1));
        }
        hit
    })
}

#[allow(clippy::too_many_arguments)]
fn text_area(
    ui: &mut egui::Ui,
    id: Id,
    text: &mut String,
    lang: Lang,
    errors: &[usize],
    hint: &str,
    keep_escape: bool,
    p: &Palette,
    font: &FontId,
) -> TextEditOutput {
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
        let mut job = match lang {
            Lang::Spec => highlight_spec(buf.as_str(), font.clone(), p, errors),
            Lang::Data => highlight_data(buf.as_str(), font.clone(), p, errors),
        };
        job.wrap.max_width = f32::INFINITY;
        ui.painter().layout_job(job)
    };
    let lines = text.split('\n').count().max(1);
    ui.horizontal_top(|ui| {
        // line-number gutter, painted after layout from the galley's own row positions
        let digits = lines.to_string().len().max(2) as f32;
        let gutter_w = digits * font.size * 0.62 + 10.0;
        let (gutter, _) = ui.allocate_exact_size(egui::vec2(gutter_w, 0.0), egui::Sense::hover());
        let gutter_x = gutter.right() - 4.0;
        let out = TextEdit::multiline(text)
            .id(id)
            .code_editor()
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::symmetric(6, 6))
            .hint_text(RichText::new(hint).font(font.clone()))
            .desired_width(f32::INFINITY)
            .desired_rows(lines.max(20))
            .layouter(&mut layouter)
            // keep focus on Escape while the popup is open (Escape closes it) or with Vim
            .event_filter(egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: keep_escape,
            })
            .show(ui);
        let name = match lang {
            Lang::Spec => "Specification editor",
            Lang::Data => "Test case editor",
        };
        let _ = out.response.response.clone().accessible_name(name);
        let mut line = 1;
        let mut new_line = true;
        for row in &out.galley.rows {
            if new_line {
                let pos = egui::pos2(gutter_x, out.galley_pos.y + row.pos.y);
                ui.painter().text(
                    pos,
                    egui::Align2::RIGHT_TOP,
                    line.to_string(),
                    font.clone(),
                    p.gutter,
                );
                line += 1;
            }
            new_line = row.ends_with_newline;
        }
        out
    })
    .inner
}

fn popup(
    ui: &egui::Ui,
    id: Id,
    comp: &Completion,
    out: &TextEditOutput,
    cursor: CCursor,
    p: &Palette,
    font: &FontId,
) -> Option<Item> {
    let rect = out
        .galley
        .pos_from_cursor(cursor)
        .translate(out.galley_pos.to_vec2());
    let mut clicked = None;
    egui::Area::new(id.with("completion"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.left_bottom() + egui::vec2(0.0, 2.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (k, it) in comp.items.iter().enumerate() {
                    let color = match it.kind {
                        Kind::Header => p.header,
                        Kind::Macro => p.macro_,
                        Kind::Keyword => p.keyword,
                        Kind::Var => p.text,
                        Kind::Type => p.ty,
                        Kind::Func => p.func,
                    };
                    let mut job = LayoutJob::default();
                    job.append(&it.label, 0.0, TextFormat::simple(font.clone(), color));
                    if !it.detail.is_empty() {
                        let small = FontId::new(font.size * 0.8, egui::FontFamily::Proportional);
                        job.append(
                            &format!("   {}", it.detail),
                            0.0,
                            TextFormat::simple(small, p.comment),
                        );
                    }
                    let resp = ui.add(
                        egui::Button::selectable(k == comp.sel, job)
                            .min_size(egui::vec2(220.0, 0.0)),
                    );
                    if resp.clicked() {
                        clicked = Some(it.clone());
                    }
                }
                ui.label(
                    RichText::new("↑↓ choose · Tab/Enter insert · Esc close")
                        .small()
                        .color(p.comment),
                );
            });
        });
    clicked
}
