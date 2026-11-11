//! The evaluation pane: verdicts, error cards and computed values.

use super::theme::{card, pane_title};
use super::{App, Tab, View};
use crate::Outcome;
use crate::diag::{Diagnostic, Pane, excerpt};
use crate::platform;
use crate::ui::{ACCENT, GREEN, I_BULB, I_FAIL, I_OK, I_RUN, I_VIEW, I_WARN, RED};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke};

const SPINNER_AFTER: f64 = 0.25;

enum Clicked {
    Test(usize),
    Position(Diagnostic, usize),
}

impl App {
    pub(super) fn eval_pane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| self.eval_header(ui));
        let frame = card(ui).inner_margin(Margin::same(10));
        if self.view != View::Results {
            frame.show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.steps_ui(ui);
            });
            return;
        }
        let mut clicked = None;
        frame.show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("eval-scroll")
                .auto_shrink(false)
                .show(ui, |ui| clicked = self.results_list(ui));
        });
        match clicked {
            Some(Clicked::Test(i)) => {
                self.current = i;
                if self.narrow {
                    self.tab = Tab::Data;
                }
            }
            Some(Clicked::Position(d, i)) => self.jump(ui.ctx(), &d, i),
            None => {}
        }
    }

    fn eval_header(&mut self, ui: &mut egui::Ui) {
        if !self.narrow {
            pane_title(ui, I_VIEW, "Evaluation");
        }
        ui.selectable_value(&mut self.view, View::Results, "Results");
        ui.selectable_value(&mut self.view, View::Stepper, "Stepper")
            .on_hover_text("Step through how the selected test case is evaluated (← / →)");
        ui.selectable_value(&mut self.view, View::StepList, "Step list");
        ui.separator();
        let now = ui.input(|i| i.time);
        let run = egui::Button::new(
            RichText::new(format!("{I_RUN}  Evaluate"))
                .strong()
                .color(Color32::WHITE),
        )
        .fill(ACCENT);
        if ui
            .add(run)
            .on_hover_text(platform::shortcut("Ctrl+Enter"))
            .clicked()
        {
            self.evaluate(now);
        }
        ui.checkbox(&mut self.auto, "Auto")
            .on_hover_text("Re-evaluate automatically shortly after every change");
        // only show the spinner for runs that take noticeably long
        if self
            .job
            .as_ref()
            .is_some_and(|j| now - j.started > SPINNER_AFTER)
        {
            ui.add(egui::Spinner::new());
            ui.label(RichText::new("Evaluating…").weak());
        }
        let (ok, bad) = self.verdicts();
        if ok > 0 {
            ui.label(RichText::new(format!("{I_OK} {ok} passed")).color(GREEN));
        }
        if bad > 0 {
            ui.label(RichText::new(format!("{I_FAIL} {bad} failed")).color(RED));
        }
        if !self.spec_diags.is_empty() {
            ui.label(
                RichText::new(format!(
                    "{I_WARN} {} problem(s) in the specification",
                    self.spec_diags.len()
                ))
                .color(RED),
            );
        }
    }

    fn results_list(&self, ui: &mut egui::Ui) -> Option<Clicked> {
        let mut clicked = None;
        if self.spec_diags.is_empty() && self.results.iter().all(Option::is_none) {
            let msg = if self.auto {
                "Results appear here as you type.".to_string()
            } else if platform::MOBILE {
                "Press Evaluate to run every test case against the specification.".to_string()
            } else {
                platform::shortcut(
                    "Press Evaluate (Ctrl+Enter) to run every test case against the specification.",
                )
            };
            ui.label(RichText::new(msg).weak());
        }
        for d in &self.spec_diags {
            if diag_ui(ui, d, &self.doc.spec, "") {
                clicked = Some(Clicked::Position(d.clone(), self.current));
            }
        }
        // failures first; within each group the selected tab leads
        let mut order: Vec<(usize, &Outcome)> = self
            .results
            .iter()
            .enumerate()
            .filter_map(|(i, o)| Some((i, o.as_ref()?)))
            .collect();
        order.sort_by_key(|(i, o)| (o.pass, *i != self.current, *i));
        for (i, o) in order {
            if let Some(c) = self.result_ui(ui, i, o) {
                clicked = Some(c);
            }
            ui.add_space(10.0);
        }
        clicked
    }

    fn result_ui(&self, ui: &mut egui::Ui, i: usize, o: &Outcome) -> Option<Clicked> {
        let mut clicked = None;
        let (icon, word, color) = if o.pass {
            (I_OK, "passed", GREEN)
        } else {
            (I_FAIL, "failed", RED)
        };
        let title = RichText::new(format!("{icon}  Test case {}  ·  {word}", i + 1))
            .strong()
            .color(color);
        let head = ui.add(egui::Label::new(title).sense(egui::Sense::click()));
        if head.on_hover_text("Show this test case").clicked() {
            clicked = Some(Clicked::Test(i));
        }
        let data = self.doc.tests.get(i).map_or("", String::as_str);
        ui.indent(("res", i), |ui| {
            for d in &o.diags {
                if diag_ui(ui, d, &self.doc.spec, data) {
                    clicked = Some(Clicked::Position(d.clone(), i));
                }
            }
            if o.values.is_empty() {
                return;
            }
            egui::Grid::new(("vals", i))
                .num_columns(2)
                .spacing([16.0, 2.0])
                .show(ui, |ui| {
                    for (name, val, from_test) in &o.values {
                        ui.label(RichText::new(name).monospace().strong());
                        let value = RichText::new(format!("= {val}")).monospace();
                        if *from_test {
                            ui.label(value);
                        } else {
                            ui.label(value.italics()).on_hover_text(
                                "What the specification gives for this input (not the test case's value)",
                            );
                        }
                        ui.end_row();
                    }
                });
        });
        clicked
    }
}

/// One diagnostic as a card. Returns true if the user clicked "go to".
fn diag_ui(ui: &mut egui::Ui, d: &Diagnostic, spec: &str, data: &str) -> bool {
    let mut go = false;
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(RED.linear_multiply(if dark { 0.10 } else { 0.07 }))
        .stroke(Stroke::new(1.0, RED.linear_multiply(0.45)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(I_WARN).color(RED));
                ui.label(RichText::new(&d.title).strong());
                if d.line > 0 {
                    let where_ = if d.pane == Pane::Data {
                        "test data"
                    } else {
                        "spec"
                    };
                    if ui
                        .link(format!("{where_} {}:{}", d.line, d.col))
                        .on_hover_text("Go to this position")
                        .clicked()
                    {
                        go = true;
                    }
                }
                ui.hyperlink_to("Docs", d.doc_url())
                    .on_hover_text("Open the part of the book that explains this");
            });
            let src = if d.pane == Pane::Data { data } else { spec };
            if let Some(line) = src.lines().nth(d.line.wrapping_sub(1)) {
                let (line, col) = excerpt(line, d.col);
                let num = d.line.to_string();
                let code = format!(
                    "{num} │ {line}\n{} │ {}^",
                    " ".repeat(num.len()),
                    " ".repeat(col - 1)
                );
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new(code).monospace());
                    });
            }
            if !d.explanation.is_empty() {
                ui.label(&d.explanation);
            }
            if let Some(h) = &d.hint {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{I_BULB} Fix:"))
                            .strong()
                            .color(ACCENT),
                    );
                    ui.label(h);
                });
            }
        });
    ui.add_space(4.0);
    go
}
