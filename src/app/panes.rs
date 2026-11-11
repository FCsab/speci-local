//! The specification editor and the test case editor.

use super::theme::{card, pane_title};
use super::{App, Tab, char_index, data_editor_id, spec_editor_id};
use crate::diag::{Diagnostic, Pane};
use crate::editor::{self, Lang, code_editor, cursor_of, replace, scan_symbols, set_cursor};
use crate::ui::{ACCENT, GREEN, I_ADD, I_DATA, I_DEL, I_PASTE, I_SPEC, I_WAND, RED};
use crate::{parse, platform, scaffold};
use eframe::egui::{self, Color32, CornerRadius, Id, RichText, Stroke};

/// Replace the selection of editor `id` (or insert at its cursor) with `s`.
fn insert_at_cursor(ctx: &egui::Context, id: Id, text: &mut String, s: &str) {
    let n = text.chars().count();
    let (a, b) = cursor_of(ctx, id).unwrap_or((n, n));
    replace(ctx, id, text, a, b, s);
}

/// Touch screens have no Ctrl+V: paste the system clipboard at the cursor.
fn paste_button(ui: &mut egui::Ui, id: Id, text: &mut String) {
    if ui
        .button(I_PASTE)
        .on_hover_text("Paste at the cursor")
        .clicked()
        && let Some(clip) = platform::paste()
    {
        insert_at_cursor(ui.ctx(), id, text, &clip.replace("\r\n", "\n"));
    }
}

impl App {
    pub(super) fn jump(&mut self, ctx: &egui::Context, d: &Diagnostic, test: usize) {
        if d.line == 0 {
            return;
        }
        let (id, text) = match d.pane {
            Pane::Data => {
                let Some(text) = self.doc.tests.get(test) else {
                    return;
                };
                self.current = test;
                self.tab = Tab::Data;
                (data_editor_id(test), text)
            }
            _ => {
                self.tab = Tab::Spec;
                (spec_editor_id(), &self.doc.spec)
            }
        };
        set_cursor(ctx, id, char_index(text, d.line, d.col));
    }

    fn refresh_symbols(&mut self) {
        if self.symbols.0 != self.doc.spec {
            self.symbols = (self.doc.spec.clone(), scan_symbols(&self.doc.spec));
        }
    }

    /// Lines to tint red in the spec editor and in test tab `test`'s data editor.
    fn error_lines(&self, pane: Pane, test: usize) -> Vec<usize> {
        let Some(ev) = &self.evaluated else {
            return vec![];
        };
        let unchanged = ev.spec == self.doc.spec
            && (pane == Pane::Spec || ev.tests.get(test) == self.doc.tests.get(test));
        if !unchanged {
            return vec![];
        }
        let of_test = self.results.get(test).into_iter().flatten();
        self.spec_diags
            .iter()
            .chain(of_test.flat_map(|o| &o.diags))
            .filter(|d| d.pane == pane && d.line > 0)
            .map(|d| d.line)
            .collect()
    }

    fn symbol_buttons(&mut self, ui: &mut egui::Ui, rtl: bool) {
        let id = spec_editor_id();
        let mut keys: Vec<_> = editor::SYMBOL_KEYS.iter().enumerate().collect();
        if rtl {
            keys.reverse(); // so they still read ∈ ∀ ∃ ≠ ≤ ≥
        }
        for (n, (_, sym)) in keys {
            if ui
                .button(RichText::new(*sym).monospace())
                .on_hover_text(platform::shortcut(&format!("Insert {sym} (Alt+{})", n + 1)))
                .clicked()
            {
                insert_at_cursor(ui.ctx(), id, &mut self.doc.spec, sym);
            }
        }
        if platform::MOBILE {
            paste_button(ui, id, &mut self.doc.spec);
        }
    }

    pub(super) fn spec_pane(&mut self, ui: &mut egui::Ui) {
        if self.narrow {
            ui.horizontal_wrapped(|ui| self.symbol_buttons(ui, false));
        } else {
            ui.horizontal(|ui| {
                pane_title(ui, I_SPEC, "Specification");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.symbol_buttons(ui, true)
                });
            });
        }
        self.refresh_symbols();
        let errors = self.error_lines(Pane::Spec, self.current);
        card(ui).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("spec-scroll")
                .auto_shrink(false)
                .show(ui, |ui| {
                    code_editor(
                        ui,
                        spec_editor_id(),
                        &mut self.doc.spec,
                        Lang::Spec,
                        &self.symbols.1,
                        &errors,
                        &mut self.comp,
                        "Be:\nn∈N\n\nKi:\n…",
                        self.vim.then_some(&mut self.vim_state),
                    );
                });
        });
    }

    fn scaffold_test(&mut self) {
        match parse(&self.doc.spec) {
            Ok(spec) => {
                let t = &mut self.doc.tests[self.current];
                if !t.is_empty() && !t.ends_with('\n') {
                    t.push('\n');
                }
                let lines = scaffold(&spec, t);
                t.push_str(&lines);
            }
            Err(d) => {
                self.spec_diags = d;
                self.results.clear();
                self.evaluated = Some(self.doc.clone());
                self.status = "Fix the specification first, then scaffold".into();
            }
        }
    }

    pub(super) fn delete_test(&mut self) {
        if self.doc.tests.len() < 2 {
            return;
        }
        let i = self.current;
        self.doc.tests.remove(i);
        if i < self.results.len() {
            self.results.remove(i);
        }
        if let Some(ev) = &mut self.evaluated
            && i < ev.tests.len()
        {
            ev.tests.remove(i);
        }
        self.current = i.min(self.doc.tests.len() - 1);
    }

    fn test_tabs(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 4.0;
        for i in 0..self.doc.tests.len() {
            let (fill, text) = match self.results.get(i) {
                Some(Some(o)) if o.pass => (GREEN, Color32::WHITE),
                Some(Some(_)) => (RED, Color32::WHITE),
                _ => (
                    ui.visuals().widgets.inactive.weak_bg_fill,
                    ui.visuals().text_color(),
                ),
            };
            let stroke = if i == self.current {
                Stroke::new(2.0, ACCENT)
            } else {
                Stroke::NONE
            };
            let b = egui::Button::new(RichText::new((i + 1).to_string()).strong().color(text))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(12))
                .min_size(egui::vec2(34.0, 0.0));
            if ui.add(b).clicked() {
                self.current = i;
            }
        }
        if ui.button(I_ADD).on_hover_text("Add a test case").clicked() {
            self.doc.tests.push(String::new());
            self.current = self.doc.tests.len() - 1;
        }
        if self.doc.tests.len() > 1
            && ui
                .button(I_DEL)
                .on_hover_text("Delete this test case")
                .clicked()
        {
            self.delete_test();
        }
    }

    pub(super) fn data_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if !self.narrow {
                pane_title(ui, I_DATA, "Test cases");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if platform::MOBILE {
                    let id = data_editor_id(self.current);
                    paste_button(ui, id, &mut self.doc.tests[self.current]);
                }
                if ui
                    .button(format!("{I_WAND} Scaffold"))
                    .on_hover_text("Insert `name: ` lines for every declared variable\n(Shift+Space, Tab in the ELTE editor)")
                    .clicked()
                {
                    self.scaffold_test();
                }
            });
        });
        ui.horizontal_wrapped(|ui| self.test_tabs(ui));
        let cur = self.current;
        self.refresh_symbols();
        let errors = self.error_lines(Pane::Data, cur);
        card(ui).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt(("data-scroll", cur))
                .auto_shrink(false)
                .show(ui, |ui| {
                    code_editor(
                        ui,
                        data_editor_id(cur),
                        &mut self.doc.tests[cur],
                        Lang::Data,
                        &self.symbols.1,
                        &errors,
                        &mut self.comp,
                        "n: 3\nnums: [1, 2, 3]",
                        self.vim.then_some(&mut self.vim_state),
                    );
                });
        });
    }
}
