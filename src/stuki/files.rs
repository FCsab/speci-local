//! Structogram files: new, open, save, and the "unsaved changes" dialog.

use super::model::Program;
use super::{Pending, Stuki, format};
use crate::platform::{self, FileKind};
use crate::ui::{self, Unsaved};
use eframe::egui;
use std::path::{Path, PathBuf};

impl Stuki {
    /// Run `action`, first asking what to do with unsaved changes.
    pub fn request(&mut self, action: Pending) {
        if self.dirty() {
            self.confirm = Some(action);
        } else {
            self.run(action);
        }
    }

    fn run(&mut self, action: Pending) {
        match action {
            Pending::New => {
                self.reset(Program::default(), None, true);
                self.status = "New structogram".into();
            }
            Pending::Open => {
                #[cfg(not(target_os = "android"))]
                if let Some(p) = crate::platform::pick_open(FileKind::Stuki) {
                    self.load(&p);
                }
                #[cfg(target_os = "android")]
                {
                    self.file_request = Some((false, String::new()));
                }
            }
            Pending::Load(p) => self.load(&p),
            Pending::Import(p) => {
                self.reset(p, None, false);
                self.status =
                    "Structogram read from the clipboard (not saved to a file yet)".into();
            }
            Pending::FromSpec(p, message) => {
                self.reset(p, None, false);
                self.status = message;
            }
        }
    }

    pub fn load(&mut self, path: &Path) {
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| format::from_st(&t))
        {
            Ok(p) => {
                self.reset(p, Some(path.to_path_buf()), true);
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    /// File name to suggest: the current file's, or the first structogram's name.
    pub(super) fn suggested_name(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .or_else(|| Some(self.program.diagrams.first()?.name.text.clone()))
            .unwrap_or_default();
        ui::file_stem(&name, "structogram")
    }

    pub fn save(&mut self, as_new: bool) {
        if let (Some(p), false) = (&self.path, as_new) {
            return self.save_to(p.clone());
        }
        let name = self.suggested_name();
        #[cfg(not(target_os = "android"))]
        if let Some(p) = crate::platform::pick_save(&name, FileKind::Stuki) {
            self.save_to(p);
        }
        #[cfg(target_os = "android")]
        {
            self.file_request = Some((true, name));
        }
    }

    pub fn save_to(&mut self, path: PathBuf) {
        let path = FileKind::Stuki.with_ext(path);
        match platform::write_file(&path, format::to_st(&self.program).as_bytes()) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.saved = self.program.content();
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not save {}: {e}", path.display()),
        }
    }

    pub fn copy_project(&mut self, ctx: &egui::Context) {
        let text = format::to_compact(&self.program);
        self.status = format!(
            "Copied the whole structogram project to the clipboard ({} characters)",
            text.len()
        );
        ctx.copy_text(text);
    }

    /// The "unsaved changes" dialog.
    pub fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else {
            return;
        };
        let Some(choice) = ui::unsaved_dialog(ctx, "stuki-discard-dialog", "structogram") else {
            return;
        };
        self.confirm = None;
        match choice {
            Unsaved::Save => {
                self.save(false);
                if !self.dirty() {
                    self.run(action);
                } else if self.file_request.is_some() {
                    self.after_save = Some(action); // Android: continue once saved
                }
            }
            Unsaved::Discard => self.run(action),
            Unsaved::Cancel => {}
        }
    }
}
