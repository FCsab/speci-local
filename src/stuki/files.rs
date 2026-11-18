//! The structograms' part of the project: they are saved with the specification in its `.sp`
//! file (see [`crate::file`]), so the app's file actions call these.

use super::model::Program;
use super::{Stuki, format};
use crate::ui;
use eframe::egui;
use std::path::Path;

impl Stuki {
    /// The structograms of a project that was opened, started or saved: `saved` marks them as
    /// what is on disk. The undo history starts again.
    pub fn reset(&mut self, program: Program, saved: bool) {
        self.saved = if saved {
            program.content()
        } else {
            String::new()
        };
        self.program = program;
        self.undo.clear();
        self.redo.clear();
        self.after_history();
        self.focus_diagram = None;
        self.last_text = None;
        self.editing = None;
    }

    /// The project was saved: the structograms on disk are the current ones.
    pub fn mark_saved(&mut self) {
        self.saved = self.program.content();
    }

    /// New structograms in place of the current ones (pasted, built from the specification or
    /// read from a `.st` file), as one step that Ctrl+Z undoes.
    pub fn replace(&mut self, program: Program, message: impl Into<String>) {
        self.push_undo();
        self.program = program;
        self.selected = None;
        self.drag = None;
        self.focus_diagram = None;
        self.last_text = None;
        self.editing = None;
        self.status = message.into();
    }

    /// Read the structograms of a `.st` file (the web Stuki's format) into the project.
    pub fn import_st(&mut self, path: &Path) {
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| format::from_st(&t))
        {
            Ok(p) => self.replace(
                p,
                format!(
                    "Structograms read from {} into the project (Ctrl+Z undoes; saving writes them to the .sp file)",
                    path.display()
                ),
            ),
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    /// File name to suggest for pictures: the first structogram's name.
    pub(super) fn suggested_name(&self) -> String {
        let name = self
            .program
            .diagrams
            .first()
            .map(|d| d.name.text.clone())
            .unwrap_or_default();
        ui::file_stem(&name, "structogram")
    }

    pub fn copy_project(&mut self, ctx: &egui::Context) {
        let text = format::to_compact(&self.program);
        self.status = format!(
            "Copied the structograms to the clipboard as one line ({} characters)",
            text.len()
        );
        ctx.copy_text(text);
    }
}
