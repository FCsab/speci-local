//! The steps of an evaluation, recorded for the stepper.

/// One step of the evaluation, for the stepper.
#[derive(Clone, Debug)]
pub struct Step {
    /// The formula being evaluated (byte span in the source) and its label, e.g. `Uf:` or `kisebb`.
    pub root: (usize, usize),
    pub label: String,
    /// The sub-expression this step evaluated (byte span in the source).
    pub span: (usize, usize),
    pub kind: StepKind,
    /// Function-call nesting depth.
    pub depth: usize,
    /// Loop variables and parameters visible at this step.
    pub scope: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
    /// The sub-expression evaluated to this value.
    Value(String),
    /// Evaluation failed here.
    Error(String),
    /// Entering a function call (`↓`), e.g. `kisebb(3)`.
    Enter(String),
    /// Returning from a function (`↑`) with this value.
    Leave(String),
}

#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub steps: Vec<Step>,
    /// More steps happened than were recorded.
    pub truncated: bool,
}

pub const TRACE_CAP: usize = 5000;
