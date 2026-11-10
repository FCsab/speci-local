//! Evaluating the document on a worker thread.

use super::App;
use crate::diag::Diagnostic;
use crate::eval::Trace;
use crate::file::Doc;
use crate::parser::Spec;
use crate::{Outcome, parse, run_and_trace, run_test_with, trace_test};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

struct EvalResult {
    outcomes: Result<Vec<Outcome>, Vec<Diagnostic>>,
    /// Steps of the selected test case, for the stepper.
    trace: Option<Trace>,
}

/// An evaluation running on a worker thread.
pub(super) struct Job {
    doc: Doc,
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<EvalResult>,
    pub(super) started: f64,
}

impl Drop for Job {
    fn drop(&mut self) {
        // a newer edit superseded this run: let the worker stop early
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub(super) fn doc_hash(d: &Doc) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    (&d.spec, &d.tests).hash(&mut h);
    h.finish()
}

fn run_all(
    spec: &Spec,
    tests: &[String],
    traced: usize,
    known: &[(String, Outcome)],
    cancel: &AtomicBool,
) -> (Vec<Outcome>, Option<Trace>) {
    let mut trace = None;
    let outcomes = tests
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let known = known.iter().find(|(text, _)| text == t);
            match known {
                Some((_, outcome)) => {
                    if i == traced {
                        trace = trace_test(spec, t, cancel);
                    }
                    outcome.clone()
                }
                None if i == traced => {
                    let (outcome, steps) = run_and_trace(spec, t, cancel);
                    trace = steps;
                    outcome
                }
                None => run_test_with(spec, t, cancel),
            }
        })
        .collect();
    (outcomes, trace)
}

impl App {
    fn known_outcomes(&self, spec: &str) -> Vec<(String, Outcome)> {
        match &self.evaluated {
            Some(ev) if ev.spec == spec && self.spec_diags.is_empty() => ev
                .tests
                .iter()
                .zip(&self.results)
                .filter_map(|(text, outcome)| Some((text.clone(), outcome.clone()?)))
                .collect(),
            _ => vec![],
        }
    }

    /// Start evaluating the current document in the background (replacing any running evaluation).
    pub(super) fn evaluate(&mut self, now: f64) {
        let doc = self.doc.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let (d, c) = (doc.clone(), cancel.clone());
        let known = self.known_outcomes(&doc.spec);
        let test = self.current;
        self.traced_test = test;
        let spawned = std::thread::Builder::new()
            .stack_size(crate::eval::EVAL_STACK)
            .spawn(move || {
                let (outcomes, trace) = match parse(&d.spec) {
                    Err(diags) => (Err(diags), None),
                    Ok(spec) => {
                        let (outcomes, trace) = run_all(&spec, &d.tests, test, &known, &c);
                        (Ok(outcomes), trace)
                    }
                };
                if !c.load(Ordering::Relaxed) {
                    let _ = tx.send(EvalResult { outcomes, trace });
                }
            });
        self.submitted = doc_hash(&doc);
        match spawned {
            Ok(_) => {
                self.job = Some(Job {
                    doc,
                    cancel,
                    rx,
                    started: now,
                })
            }
            Err(e) => self.status = format!("Could not start the evaluation: {e}"),
        }
    }

    /// Pick up a finished background evaluation.
    pub(super) fn poll_job(&mut self) {
        let Some(job) = &self.job else { return };
        let r = match job.rx.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = "The evaluation crashed (this is a bug in speci-local)".into();
                self.job = None;
                return;
            }
        };
        let job = self.job.take().unwrap();
        self.trace = r.trace;
        match r.outcomes {
            Err(d) => {
                self.spec_diags = d;
                self.results = vec![None; job.doc.tests.len()];
                self.status = "The specification has errors".into();
            }
            Ok(outcomes) => {
                self.spec_diags.clear();
                let ok = outcomes.iter().filter(|o| o.pass).count();
                self.status = format!("{ok} of {} test case(s) passed", outcomes.len());
                self.results = outcomes.into_iter().map(Some).collect();
            }
        }
        self.evaluated = Some(job.doc.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "In: n∈N\nOut: d∈N\nPre: -\nPost: d=2*n";

    #[test]
    fn known_verdicts_are_not_computed_again() {
        let spec = parse(SPEC).unwrap();
        let tests: Vec<String> = ["n: 1\nd: 2", "n: 2\nd: 5", "n: 3\nd: 6"]
            .map(String::from)
            .to_vec();
        let cancel = AtomicBool::new(false);
        let (fresh, trace) = run_all(&spec, &tests, 1, &[], &cancel);
        assert_eq!(
            fresh.iter().map(|o| o.pass).collect::<Vec<_>>(),
            [true, false, true]
        );
        let steps = trace.expect("the second test case's steps").steps;
        assert!(
            steps
                .iter()
                .any(|s| s.kind == crate::eval::StepKind::Value("4".into()))
        );

        let mut marked = fresh[1].clone();
        marked.diags[0].title = "remembered".into();
        let known = vec![(tests[2].clone(), marked)];
        let (again, trace) = run_all(&spec, &tests, 2, &known, &cancel);
        assert!(again[0].pass && !again[1].pass);
        assert_eq!(again[2].diags[0].title, "remembered");
        assert!(trace.is_some_and(|t| !t.steps.is_empty()));
        let edited = vec!["n: 3\nd: 7".to_string()];
        let (o, _) = run_all(&spec, &edited, 0, &known, &cancel);
        assert_ne!(o[0].diags[0].title, "remembered");
    }

    #[test]
    fn broken_test_data_has_a_verdict_but_no_steps() {
        let spec = parse(SPEC).unwrap();
        let tests = vec!["n: [".to_string()];
        let (o, trace) = run_all(&spec, &tests, 0, &[], &AtomicBool::new(false));
        assert!(!o[0].pass && trace.is_none());
    }
}
