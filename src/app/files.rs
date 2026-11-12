//! The document's life cycle: new, open, save, and projects copied through the clipboard.

use super::{App, Replace, Tab, Tool};
use crate::editor::Completion;
use crate::file::{Doc, from_clipboard, from_sp, to_compact, to_sp};
use crate::platform::{self, FileKind};
use crate::stuki::{self, Pending};
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
    pub(super) fn dirty(&self) -> bool {
        self.doc != self.saved
    }

    pub(super) fn file_new(&mut self) {
        match self.tool {
            Tool::Spec => self.request(Replace::New),
            Tool::Stuki => self.stuki.request(Pending::New),
        }
    }

    pub(super) fn file_open(&mut self) {
        match self.tool {
            Tool::Spec => self.request(Replace::Open),
            Tool::Stuki => self.stuki.request(Pending::Open),
        }
    }

    pub(super) fn file_save(&mut self, as_new: bool) {
        match self.tool {
            Tool::Spec => self.save(as_new),
            Tool::Stuki => self.stuki.save(as_new),
        }
    }

    pub(super) fn open_path(&mut self, path: PathBuf) {
        if FileKind::of(&path) == FileKind::Stuki {
            self.tool = Tool::Stuki;
            self.stuki.request(Pending::Load(path));
        } else {
            self.tool = Tool::Spec;
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
                self.tab = Tab::Spec;
            }
            Replace::Open => {
                #[cfg(not(target_os = "android"))]
                if let Some(p) = platform::pick_open(FileKind::Spec) {
                    self.load(&p);
                }
                #[cfg(target_os = "android")]
                {
                    self.files = Some(super::android_files::FileDialog::new(
                        false,
                        String::new(),
                        "sp",
                    ));
                }
            }
            Replace::Load(p) => self.load(&p),
            Replace::Import(doc) => {
                self.reset_doc(doc, None);
                self.saved = Doc::default(); // imported, not saved anywhere yet
                self.tab = Tab::Spec;
                self.status = "Project read from the clipboard (not saved to a file yet)".into();
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

    pub(super) fn load(&mut self, path: &Path) {
        if FileKind::of(path) == FileKind::Stuki {
            self.tool = Tool::Stuki;
            self.stuki.request(Pending::Load(path.to_path_buf()));
            return;
        }
        self.tool = Tool::Spec;
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut doc = from_sp(&text);
                if doc.title.is_empty() {
                    doc.title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                }
                self.reset_doc(doc, Some(path.to_path_buf()));
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    pub(super) fn save(&mut self, as_new: bool) {
        if let (Some(p), false) = (&self.path, as_new) {
            return self.save_to(p.clone());
        }
        let name = ui::file_stem(&self.doc.title, "specification");
        #[cfg(not(target_os = "android"))]
        if let Some(path) = platform::pick_save(&name, FileKind::Spec) {
            self.save_to(path);
        }
        #[cfg(target_os = "android")]
        {
            self.files = Some(super::android_files::FileDialog::new(true, name, "sp"));
        }
    }

    pub(super) fn save_to(&mut self, path: PathBuf) {
        let path = FileKind::Spec.with_ext(path);
        match platform::write_file(&path, to_sp(&self.doc).as_bytes()) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.saved = self.doc.clone();
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Could not save {}: {e}", path.display()),
        }
    }

    pub(super) fn copy_project(&mut self, ctx: &egui::Context) {
        let text = to_compact(&self.doc);
        self.status = format!(
            "Copied the whole project to the clipboard ({} characters)",
            text.len()
        );
        ctx.copy_text(text);
    }

    pub(super) fn paste_project(&mut self) {
        match platform::paste() {
            Some(text) => self.import_text(&text),
            // e.g. no clipboard access on this desktop: let the user paste into a text box
            None => self.paste_box = Some(String::new()),
        }
    }

    /// Auto Stuki: structograms built from the specification replace the current ones.
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
        self.stuki.request(Pending::FromSpec(g.program, message));
    }

    /// Read a copied project: a structogram goes to the Stuki tab, a specification to the other.
    pub(super) fn import_text(&mut self, text: &str) {
        if stuki::format::looks_like_stuki(text) {
            self.tool = Tool::Stuki;
            match stuki::format::from_clipboard(text) {
                Ok(p) => self.stuki.request(Pending::Import(p)),
                Err(e) => self.stuki.status = e,
            }
            return;
        }
        match from_clipboard(text) {
            Ok(doc) => {
                self.tool = Tool::Spec;
                self.request(Replace::Import(doc));
            }
            Err(_) if self.tool == Tool::Stuki => {
                self.stuki.status = "The clipboard doesn't contain a project: expected text starting with STUKI1: (or SPECI1:), a .st file's contents, or a Stuki link with data=.".into();
            }
            Err(e) => self.status = e,
        }
    }
}
