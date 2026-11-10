//! speci-local: an offline editor for ELTE-style specifications (with test cases) and
//! structograms.

pub mod app;
mod compact;
pub mod data;
pub mod diag;
mod editor;
pub mod eval;
pub mod file;
pub mod parser;
pub mod platform;
mod runner;
pub mod stuki;
mod ui;

pub use parser::parse;
pub use runner::{Outcome, run_and_trace, run_test, run_test_with, scaffold, trace_test};
