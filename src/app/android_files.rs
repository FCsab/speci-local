//! Android's stand-in for the native Open / Save as dialogs: the files in the app's own folder.

use super::App;
use crate::platform;
use crate::ui::{I_SAVE, I_SPEC, I_STUKI, modal_width};
use eframe::egui::{self, Id, RichText, TextEdit};
use std::path::PathBuf;

/// Android's stand-in for the Open / Save as dialogs: the projects (`.sp`) in the app's folder,
/// and for Open also the web Stuki's `.st` files.
pub(super) struct FileDialog {
    save: bool,
    /// file name being saved as (without `.sp`)
    name: String,
    dir: Option<PathBuf>,
    files: Vec<PathBuf>,
    /// what to do once the file is saved ("Save first" in the unsaved-changes dialog)
    pub(super) then: Option<super::Replace>,
}

impl FileDialog {
    pub(super) fn new(save: bool, name: String) -> Self {
        let exts: &[&str] = if save { &["sp"] } else { &["sp", "st"] };
        let dir = platform::documents_dir();
        let mut files: Vec<PathBuf> = dir
            .as_ref()
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| exts.iter().any(|x| e == *x)))
            .collect();
        files.sort();
        FileDialog {
            save,
            name,
            dir,
            files,
            then: None,
        }
    }
}

impl App {
    /// Android: pick a file to open, or a name to save the project as.
    pub(super) fn files_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.files.take() else {
            return;
        };
        let (mut open, mut save, mut cancel) = (None, false, false);
        let modal = egui::Modal::new(Id::unique("files-dialog")).show(ctx, |ui| {
            ui.set_width(modal_width(ctx, 520.0));
            ui.heading(if d.save { "Save as" } else { "Open" });
            if d.save {
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(&mut d.name)
                            .hint_text("name")
                            .desired_width(ui.available_width() - 40.0),
                    );
                    ui.label(".sp");
                });
            }
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    if d.files.is_empty() {
                        ui.label(RichText::new("No saved projects yet.").weak());
                    }
                    for f in &d.files {
                        let stem = f.file_stem().unwrap_or_default().to_string_lossy();
                        let chosen = d.save && stem == d.name.trim();
                        let st = f.extension().is_some_and(|e| e == "st");
                        let (icon, label) = if st {
                            (I_STUKI, format!("{stem}.st"))
                        } else {
                            (I_SPEC, stem.to_string())
                        };
                        if ui
                            .selectable_label(chosen, format!("{icon}  {label}"))
                            .clicked()
                        {
                            if d.save {
                                d.name = stem.into_owned();
                            } else {
                                open = Some(f.clone());
                            }
                        }
                    }
                });
            if let Some(dir) = &d.dir {
                ui.label(RichText::new(dir.display().to_string()).small().weak());
            }
            ui.horizontal(|ui| {
                let name = d.name.trim();
                if d.save {
                    let exists = d
                        .files
                        .iter()
                        .any(|f| f.file_stem().is_some_and(|s| s == name));
                    let label = if exists { "Replace" } else { "Save" };
                    let ok = !name.is_empty() && !name.contains(['/', '\\']) && d.dir.is_some();
                    if ui
                        .add_enabled(ok, egui::Button::new(format!("{I_SAVE}  {label}")))
                        .clicked()
                    {
                        save = true;
                    }
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if let Some(p) = open {
            self.load(&p); // already asked about unsaved changes
        } else if save {
            if let Some(dir) = &d.dir {
                self.save_to(dir.join(format!("{}.sp", d.name.trim())));
            }
            if let Some(next) = d.then.take()
                && !self.dirty()
            {
                self.request(next);
            }
        } else if !cancel && !modal.should_close() {
            self.files = Some(d);
        }
    }
}
