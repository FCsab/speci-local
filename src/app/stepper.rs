//! Stepper and step list: how the selected test case is evaluated, one sub-expression at a time
//! (the original editor's visualization pane).

use super::{App, View};
use crate::eval::{Step, StepKind, Trace};
use crate::ui::{ACCENT, I_FIRST, I_LAST, I_NEXT, I_PREV, RED};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, TextStyle};

const STEP_BG: Color32 = Color32::from_rgb(250, 204, 21);
const SNIPPET_CHARS: usize = 70;

impl App {
    pub(super) fn steps_ui(&mut self, ui: &mut egui::Ui) {
        let Some(trace) = &self.trace else {
            let msg = if self.spec_diags.is_empty() {
                "No steps yet. They appear once the selected test case's data is valid."
            } else {
                "Fix the specification first; the steps appear once it parses."
            };
            ui.label(RichText::new(msg).weak());
            return;
        };
        let n = trace.steps.len();
        if n == 0 {
            ui.label(
                RichText::new("Nothing to step through: the conditions contain only constants.")
                    .weak(),
            );
            return;
        }
        let src = self.evaluated.as_ref().map_or("", |d| d.spec.as_str());
        self.step = self.step.min(n - 1);
        let mut note = format!("Test case {}", self.traced_test + 1);
        if trace.truncated {
            note += &format!(" · only the first {n} steps are shown");
        }
        if self.view == View::StepList {
            if let Some(chosen) = step_list(ui, trace, src, self.step, &note) {
                self.step = chosen;
                self.view = View::Stepper;
            }
        } else {
            self.step = stepper(ui, trace, src, self.step, &note);
        }
    }
}

fn snippet(src: &str, span: (usize, usize)) -> String {
    let t = src
        .get(span.0..span.1)
        .unwrap_or("?")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if t.chars().count() > SNIPPET_CHARS {
        let start: String = t.chars().take(SNIPPET_CHARS - 1).collect();
        format!("{start}…")
    } else {
        t
    }
}

fn describe(src: &str, st: &Step) -> String {
    match &st.kind {
        StepKind::Value(v) => format!("{}  =  {v}", snippet(src, st.span)),
        StepKind::Error(t) => format!("{}  ✗  {t}", snippet(src, st.span)),
        StepKind::Enter(c) => format!("↓ {c}"),
        StepKind::Leave(r) => format!("↑ {r}"),
    }
}

fn step_list(
    ui: &mut egui::Ui,
    trace: &Trace,
    src: &str,
    current: usize,
    note: &str,
) -> Option<usize> {
    ui.label(RichText::new(note).weak());
    let row_h = ui.text_style_height(&TextStyle::Monospace) + 6.0;
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt("step-list")
        .auto_shrink(false)
        .show_rows(ui, row_h, trace.steps.len(), |ui, range| {
            for i in range {
                let st = &trace.steps[i];
                let text = format!(
                    "{:>5}  {}{:<5} {}",
                    i + 1,
                    "  ".repeat(st.depth),
                    st.label,
                    describe(src, st)
                );
                let mut rt = RichText::new(text).monospace();
                if matches!(st.kind, StepKind::Error(_)) {
                    rt = rt.color(RED);
                }
                if ui
                    .selectable_label(i == current, rt)
                    .on_hover_text("Show this step in the stepper")
                    .clicked()
                {
                    chosen = Some(i);
                }
            }
        });
    chosen
}

fn stepper(ui: &mut egui::Ui, trace: &Trace, src: &str, step: usize, note: &str) -> usize {
    let step = step_controls(ui, step, trace.steps.len(), note);
    let st = &trace.steps[step];
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(&st.label).strong().color(ACCENT));
        if st.depth > 0 {
            ui.label(RichText::new(format!("inside a function call (depth {})", st.depth)).weak());
        }
    });
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::horizontal()
                .id_salt("step-formula")
                .show(ui, |ui| ui.label(formula(ui, src, st)));
        });
    ui.add_space(4.0);
    let color = match &st.kind {
        StepKind::Error(_) => RED,
        StepKind::Enter(_) | StepKind::Leave(_) => ACCENT,
        StepKind::Value(_) => ui.visuals().strong_text_color(),
    };
    ui.label(RichText::new(describe(src, st)).monospace().color(color));
    if !st.scope.is_empty() {
        let vars: Vec<String> = st.scope.iter().map(|(n, v)| format!("{n} = {v}")).collect();
        ui.label(RichText::new(vars.join("   ")).monospace().weak());
    }
    step
}

fn step_controls(ui: &mut egui::Ui, mut step: usize, n: usize, note: &str) -> usize {
    let (prev, next) = (step.saturating_sub(1), (step + 1).min(n - 1));
    if ui.memory(|m| m.focused().is_none()) {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                step = next;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                step = prev;
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        for (icon, tip, target) in [
            (I_FIRST, "First step", 0),
            (I_PREV, "Previous step (←)", prev),
            (I_NEXT, "Next step (→)", next),
            (I_LAST, "Last step", n - 1),
        ] {
            if ui.button(icon).on_hover_text(tip).clicked() {
                step = target;
            }
        }
        ui.spacing_mut().slider_width = (ui.available_width() - 260.0).clamp(100.0, 600.0);
        let mut one_based = step + 1;
        ui.add(egui::Slider::new(&mut one_based, 1..=n).text(format!("of {n}")));
        step = one_based - 1;
        ui.label(RichText::new(note).weak());
    });
    step
}

fn formula(ui: &egui::Ui, src: &str, st: &Step) -> egui::text::LayoutJob {
    let line_start = src
        .get(..st.root.0)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |i| i + 1);
    let line_end = src
        .get(st.root.1..)
        .and_then(|after| after.find('\n'))
        .map_or(src.len(), |i| i + st.root.1);
    let font = TextStyle::Monospace.resolve(ui.style());
    let normal = egui::TextFormat::simple(font.clone(), ui.visuals().text_color());
    let mut highlighted = egui::TextFormat::simple(font, Color32::BLACK);
    highlighted.background = STEP_BG;
    let mut job = egui::text::LayoutJob::default();
    for (a, b, format) in [
        (line_start, st.span.0, &normal),
        (st.span.0, st.span.1, &highlighted),
        (st.span.1, line_end, &normal),
    ] {
        if let Some(t) = src.get(a..b) {
            job.append(t, 0.0, format.clone());
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippets_are_one_short_line() {
        let src = "Uf: db = DARAB(i=1..n,\n      szamok[i] mod 2 = 1)";
        assert_eq!(snippet(src, (4, 11)), "db = DA");
        assert_eq!(
            snippet(src, (9, src.len())),
            "DARAB(i=1..n, szamok[i] mod 2 = 1)"
        );
        assert_eq!(snippet(src, (4, 999)), "?");
        let long = "x".repeat(200);
        let s = snippet(&long, (0, 200));
        assert_eq!(s.chars().count(), SNIPPET_CHARS);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn steps_are_described_by_kind() {
        let step = |kind| Step {
            root: (0, 3),
            label: "Uf:".into(),
            span: (0, 3),
            kind,
            depth: 0,
            scope: vec![],
        };
        let src = "a+b";
        assert_eq!(
            describe(src, &step(StepKind::Value("3".into()))),
            "a+b  =  3"
        );
        assert_eq!(
            describe(src, &step(StepKind::Error("Division by zero".into()))),
            "a+b  ✗  Division by zero"
        );
        assert_eq!(
            describe(src, &step(StepKind::Enter("f(1)".into()))),
            "↓ f(1)"
        );
        assert_eq!(
            describe(src, &step(StepKind::Leave("f → 2".into()))),
            "↑ f → 2"
        );
    }
}
