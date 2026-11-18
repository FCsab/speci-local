//! The project's life cycle: new, open, save, and projects copied through the clipboard.
//! A project is one `.sp` file: the specification, its test cases and the structograms.

use super::{App, Replace, Tab, Tool};
use crate::editor::Completion;
use crate::file::{Doc, Project, from_clipboard, from_project, from_sp, to_compact, to_project};
use crate::platform::{self, FileKind};
use crate::stuki::{self, model::Program};
use crate::ui;
use eframe::egui;
use std::path::{Path, PathBuf};

fn blank() -> Doc {
    Doc {
        title: String::new(),
        spec: "Be:\n\nKi:\n\nEf:\n-\n\nUf:\n".into(),
        tests: vec![String::new()],
    }
}

impl App {
    /// Unsaved changes anywhere in the project: specification, test cases or structograms.
    pub(super) fn dirty(&self) -> bool {
        self.doc != self.saved || self.stuki.dirty()
    }

    pub(super) fn file_new(&mut self) {
        self.request(Replace::New);
    }

    pub(super) fn file_open(&mut self) {
        self.request(Replace::Open);
    }

    pub(super) fn file_save(&mut self, as_new: bool) {
        self.save(as_new);
    }

    /// A `.st` file's structograms come into the current project (an undoable change); anything
    /// else is opened as a project.
    pub(super) fn open_path(&mut self, path: PathBuf) {
        if FileKind::of(&path) == FileKind::Stuki {
            self.tool = Tool::Stuki;
            self.stuki.import_st(&path);
        } else {
            self.request(Replace::Load(path));
        }
    }

    /// Run an action that replaces the document, asking first if there are unsaved changes.
    pub(super) fn request(&mut self, action: Replace) {
        if self.dirty() {
            self.confirm = Some(action);
        } else {
            self.run(action);
        }
    }

    pub(super) fn run(&mut self, action: Replace) {
        match action {
            Replace::New => {
                self.reset_doc(blank(), None);
                self.stuki.reset(Program::default(), true);
                self.tab = Tab::Spec;
                self.stuki.status = "New project".into();
                self.status = "New project".into();
            }
            Replace::Open => {
                #[cfg(not(target_os = "android"))]
                if let Some(p) = platform::pick_open(FileKind::Spec) {
                    self.load(&p); // already asked about unsaved changes
                }
                #[cfg(target_os = "android")]
                {
                    self.files = Some(super::android_files::FileDialog::new(false, String::new()));
                }
            }
            Replace::Load(p) => self.load(&p),
            Replace::Import(project) => {
                self.reset_doc(project.doc, None);
                self.saved = Doc::default(); // imported, not saved anywhere yet
                self.stuki.reset(project.stuki.unwrap_or_default(), false);
                self.tab = Tab::Spec;
                self.status = "Project read from the clipboard (not saved to a file yet)".into();
                self.stuki.status = self.status.clone();
            }
        }
    }

    fn reset_doc(&mut self, doc: Doc, path: Option<PathBuf>) {
        self.saved = doc.clone();
        self.doc = doc;
        self.path = path;
        self.current = 0;
        self.results.clear();
        self.spec_diags.clear();
        self.evaluated = None;
        self.trace = None;
        self.step = 0;
        self.job = None;
        self.submitted = 0; // evaluated on the next frame
        self.comp = Completion::default();
        self.status.clear();
    }

    /// Open a project: its specification, test cases and structograms replace the current ones.
    pub(super) fn load(&mut self, path: &Path) {
        if FileKind::of(path) == FileKind::Stuki {
            return self.open_path(path.to_path_buf());
        }
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                let e = format!("Could not open {}: {e}", path.display());
                self.stuki.status = e.clone();
                self.status = e;
                return;
            }
        };
        let (project, problem) = match from_project(&text) {
            Ok(p) => (p, None),
            Err(e) => {
                let doc = from_sp(&text);
                (Project { doc, stuki: None }, Some(e))
            }
        };
        let mut doc = project.doc;
        if doc.title.is_empty() {
            doc.title = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
        }
        self.reset_doc(doc, Some(path.to_path_buf()));
        // structograms that can't be read are not on screen: saving would leave them out
        self.stuki
            .reset(project.stuki.unwrap_or_default(), problem.is_none());
        self.status = match problem {
            Some(e) => format!("Opened {}, but {e}", path.display()),
            None => format!("Opened {}", path.display()),
        };
        self.stuki.status = self.status.clone();
    }

    pub(super) fn save(&mut self, as_new: bool) {
        if let (Some(p), false) = (&self.path, as_new) {
            let old = p.clone();
            let Some(new) = self.renamed(&old) else {
                return self.save_to(old);
            };
            self.save_to(new.clone());
            // saved under the new name: the file with the old name goes
            if self.path.as_ref() == Some(&new)
                && let Err(e) = std::fs::remove_file(&old)
            {
                self.status += &format!("; could not remove {}: {e}", old.display());
            }
            return;
        }
        let name = ui::file_stem(&self.doc.title, "specification");
        #[cfg(not(target_os = "android"))]
        if let Some(path) = platform::pick_save(&name, FileKind::Spec) {
            self.save_to(path);
        }
        #[cfg(target_os = "android")]
        {
            self.files = Some(super::android_files::FileDialog::new(true, name));
        }
    }

    /// The file's new path when the task was renamed since the last save and the file was
    /// named after it. Another file's name is never taken.
    fn renamed(&self, path: &Path) -> Option<PathBuf> {
        let named = |title: &str| ui::file_stem(title, "specification");
        let (was, now) = (named(&self.saved.title), named(&self.doc.title));
        if was == now || path.file_stem()? != was.as_str() {
            return None;
        }
        let ext = path.extension()?.to_string_lossy();
        let new = path.with_file_name(format!("{now}.{ext}"));
        (!new.exists()).then_some(new)
    }

    /// Write the whole project (specification, test cases and structograms) into one `.sp` file.
    pub(super) fn save_to(&mut self, path: PathBuf) {
        let path = FileKind::Spec.with_ext(path);
        let text = to_project(&self.doc, &self.stuki.program);
        match platform::write_file(&path, text.as_bytes()) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.saved = self.doc.clone();
                self.stuki.mark_saved();
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not save {}: {e}", path.display()),
        }
        self.stuki.status = self.status.clone();
    }

    /// The whole project, structograms included, as one `SPECI1:` line.
    pub(super) fn copy_project(&mut self, ctx: &egui::Context) {
        let text = to_compact(&self.doc, &self.stuki.program);
        self.status = format!(
            "Copied the whole project to the clipboard ({} characters)",
            text.len()
        );
        ctx.copy_text(text);
    }

    /// Opens the current tab's project in the ELTE web editor.
    pub(super) fn open_in_web_editor(&mut self, ctx: &egui::Context) {
        ctx.open_url(egui::OpenUrl::new_tab(self.web_link()));
        let msg = "Opened the project in the web editor".to_string();
        match self.tool {
            Tool::Spec => self.status = msg,
            Tool::Stuki => self.stuki.status = msg,
        }
    }

    /// Copies a link that opens the current tab's project in the ELTE web editor.
    pub(super) fn copy_web_link(&mut self, ctx: &egui::Context) {
        ctx.copy_text(self.web_link());
        let msg = "Copied a link that opens the project in the web editor".to_string();
        match self.tool {
            Tool::Spec => self.status = msg,
            Tool::Stuki => self.stuki.status = msg,
        }
    }

    fn web_link(&self) -> String {
        match self.tool {
            Tool::Spec => crate::file::to_web_link(&self.doc),
            Tool::Stuki => stuki::format::to_web_link(&self.stuki.program),
        }
    }

    pub(super) fn paste_project(&mut self) {
        match platform::paste() {
            Some(text) => self.import_text(&text),
            // e.g. no clipboard access on this desktop: let the user paste into a text box
            None => self.paste_box = Some(String::new()),
        }
    }

    /// From spec: structograms built from the specification replace the current ones.
    pub(super) fn stuki_from_spec(&mut self) {
        let spec = match crate::parser::parse(&self.doc.spec) {
            Ok(s) => s,
            Err(d) => {
                let at = match d.first() {
                    Some(d) if d.line > 0 => format!(" (line {}: {})", d.line, d.title),
                    Some(d) => format!(" ({})", d.title),
                    None => String::new(),
                };
                self.stuki.status =
                    format!("Fix the specification on the Specification tab first{at}");
                return;
            }
        };
        let g = stuki::from_spec::from_spec(&spec, &self.doc.title);
        let mut message = "Structograms built from the specification".to_string();
        if !g.unset.is_empty() {
            message += &format!(
                "; the postcondition doesn't say how to compute {} (write it with `=` or a pattern)",
                g.unset.join(", ")
            );
        }
        self.stuki.replace(g.program, message + " (Ctrl+Z undoes)");
    }

    /// Read a copied project: a structogram goes to the Stuki tab, a specification to the other.
    pub(super) fn import_text(&mut self, text: &str) {
        if stuki::format::looks_like_stuki(text) {
            self.tool = Tool::Stuki;
            match stuki::format::from_clipboard(text) {
                Ok(p) => self.stuki.replace(
                    p,
                    "Structograms read from the clipboard into the project (Ctrl+Z undoes)",
                ),
                Err(e) => self.stuki.status = e,
            }
            return;
        }
        match from_clipboard(text) {
            Ok(project) => {
                self.tool = Tool::Spec;
                self.request(Replace::Import(project));
            }
            Err(_) if self.tool == Tool::Stuki => {
                self.stuki.status = "The clipboard doesn't contain a project: expected text starting with STUKI1: (or SPECI1:), a .st file's contents, or a Stuki link with data=.".into();
            }
            Err(e) => self.status = e,
        }
    }
}
