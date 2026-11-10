//! The test row of the Stuki tab: the structograms run on the specification's test cases.

use super::Stuki;
use super::run::{Verdict, run_tests};
use crate::ui::{GREEN, I_FAIL, I_OK, RED};
use eframe::egui::{self, RichText};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

type Outcome = Result<Vec<Verdict>, String>;

struct Job {
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<Outcome>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub(super) struct Checks {
    of: u64,
    pub(super) outcome: Option<Outcome>,
    job: Option<Job>,
    shown: Option<usize>,
}

fn run(
    program: &super::model::Program,
    spec: &str,
    tests: &[String],
    cancel: &AtomicBool,
) -> Outcome {
    match crate::parse(spec) {
        Ok(spec) => Ok(run_tests(program, &spec, tests, cancel)),
        Err(diags) => Err(match diags.first() {
            Some(d) if d.line > 0 => {
                format!("Fix the specification first (line {}: {})", d.line, d.title)
            }
            Some(d) => format!("Fix the specification first ({})", d.title),
            None => "Fix the specification first".into(),
        }),
    }
}

impl Stuki {
    fn refresh_checks(&mut self, ctx: &egui::Context, spec: &str, tests: &[String]) {
        let mut h = std::hash::DefaultHasher::new();
        (self.program.content(), spec, tests).hash(&mut h);
        let of = h.finish();
        if of != self.checks.of {
            self.checks.of = of;
            self.checks.shown = None;
            let cancel = Arc::new(AtomicBool::new(false));
            let (tx, rx) = mpsc::channel();
            let (program, spec, tests) = (self.program.clone(), spec.to_string(), tests.to_vec());
            let (c, ctx) = (cancel.clone(), ctx.clone());
            let spawned = std::thread::Builder::new()
                .stack_size(crate::eval::EVAL_STACK)
                .spawn(move || {
                    let outcome = run(&program, &spec, &tests, &c);
                    if !c.load(Ordering::Relaxed) {
                        let _ = tx.send(outcome);
                        ctx.request_repaint();
                    }
                });
            self.checks.job = match spawned {
                Ok(_) => Some(Job { cancel, rx }),
                Err(e) => {
                    self.checks.outcome = Some(Err(format!("Could not start the run: {e}")));
                    None
                }
            };
        }
        let Some(job) = &self.checks.job else { return };
        match job.rx.try_recv() {
            Ok(outcome) => {
                self.checks.outcome = Some(outcome);
                self.checks.job = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.checks.outcome =
                    Some(Err("The run crashed (this is a bug in speci-local)".into()));
                self.checks.job = None;
            }
        }
    }

    pub(super) fn tests_row(&mut self, ui: &mut egui::Ui, spec: &str, tests: &[String]) {
        self.refresh_checks(ui.ctx(), spec, tests);
        let mut clicked = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Tests").small().weak())
                .on_hover_text("The structograms, run on the test cases of the Specification tab: a test case passes if the outputs satisfy the postcondition");
            match &self.checks.outcome {
                None => {
                    ui.spinner();
                }
                Some(Err(why)) => {
                    ui.label(RichText::new(why).color(RED));
                }
                Some(Ok(verdicts)) if verdicts.is_empty() => {
                    ui.label(
                        RichText::new("The specification has no test cases yet: add them on the Specification tab")
                            .weak(),
                    );
                }
                Some(Ok(verdicts)) => {
                    let shown = self
                        .checks
                        .shown
                        .filter(|&i| i < verdicts.len())
                        .or_else(|| verdicts.iter().position(|v| !v.pass));
                    for (i, v) in verdicts.iter().enumerate() {
                        let (icon, color) = if v.pass { (I_OK, GREEN) } else { (I_FAIL, RED) };
                        let chip = egui::Button::new(
                            RichText::new(format!("{icon} {}", i + 1)).color(color),
                        )
                        .selected(shown == Some(i) && !v.pass);
                        if ui.add(chip).on_hover_text(details(i, v)).clicked() {
                            clicked = Some((i, v.node));
                        }
                    }
                    match shown {
                        Some(i) if !verdicts[i].pass => {
                            let line = verdicts[i].message.lines().next().unwrap_or_default();
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("Test {}: {line}", i + 1)).small(),
                                )
                                .truncate(),
                            )
                            .on_hover_text(details(i, &verdicts[i]));
                        }
                        _ if verdicts.iter().all(|v| v.pass) => {
                            ui.label(
                                RichText::new(format!(
                                    "All {} test case(s) pass",
                                    verdicts.len()
                                ))
                                .small()
                                .color(GREEN),
                            );
                        }
                        _ => {}
                    }
                }
            }
            if self.checks.job.is_some() && self.checks.outcome.is_some() {
                ui.spinner();
            }
        });
        if let Some((i, node)) = clicked {
            self.checks.shown = Some(i);
            if let Some(id) = node.filter(|id| self.program.find(*id).is_some()) {
                self.selected = Some(id);
            }
        }
    }
}

fn details(i: usize, v: &Verdict) -> String {
    let mut text = format!(
        "Test case {}: {}",
        i + 1,
        if v.pass { "passes" } else { "fails" }
    );
    if !v.message.is_empty() {
        text += &format!("\n{}", v.message);
    }
    if !v.values.is_empty() {
        let values: Vec<String> = v.values.iter().map(|(n, v)| format!("{n} = {v}")).collect();
        text += &format!("\nThe structogram gives: {}", values.join(", "));
    }
    if v.node.is_some() {
        text += "\nClick to select the element where the run stopped.";
    }
    text
}

#[cfg(test)]
impl Stuki {
    pub(crate) fn verdicts(&self) -> Option<Result<Vec<(bool, String)>, String>> {
        if self.checks.job.is_some() {
            return None;
        }
        Some(match self.checks.outcome.clone()? {
            Ok(v) => Ok(v.into_iter().map(|v| (v.pass, v.message)).collect()),
            Err(e) => Err(e),
        })
    }

    pub(crate) fn selected(&self) -> Option<u64> {
        self.selected
    }
}
