//! The modal dialogs: unsaved changes, and pasting a project by hand.

use super::App;
use crate::platform;
use crate::ui::{self, I_PASTE, Unsaved, modal_width};
use eframe::egui::{self, Id, TextEdit, TextStyle};

impl App {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        self.confirm_dialog(ctx);
        self.stuki.confirm_dialog(ctx);
        #[cfg(target_os = "android")]
        self.files_dialog(ctx);
        self.paste_dialog(ctx);
    }

    fn after_save(&mut self, action: super::Replace) {
        // Android: the save dialog is open, continue once it's done
        #[cfg(target_os = "android")]
        if let Some(f) = &mut self.files {
            f.then = Some(action);
        }
        #[cfg(not(target_os = "android"))]
        let _ = action;
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else {
            return;
        };
        let Some(choice) = ui::unsaved_dialog(ctx, "discard-dialog", "specification") else {
            return;
        };
        self.confirm = None;
        match choice {
            Unsaved::Save => {
                self.save(false);
                if self.dirty() {
                    self.after_save(action);
                } else {
                    self.run(action);
                }
            }
            Unsaved::Discard => self.run(action),
            Unsaved::Cancel => {}
        }
    }

    fn paste_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.paste_box.take() else {
            return;
        };
        let mut done = false;
        let modal = egui::Modal::new(Id::unique("paste-dialog")).show(ctx, |ui| {
            ui.set_width(modal_width(ctx, 520.0));
            ui.heading("Paste a project");
            ui.label(platform::shortcut("The clipboard couldn't be read directly. Paste the copied project (SPECI1:…) or a specification here with Ctrl+V:"));
            ui.add(TextEdit::multiline(&mut text).desired_rows(6).desired_width(f32::INFINITY).font(TextStyle::Monospace));
            ui.horizontal(|ui| {
                if ui.button(format!("{I_PASTE}  Import")).clicked() {
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    text.clear();
                    done = true;
                }
            });
        });
        if modal.should_close() && !done {
            return; // closed with Escape or a click outside
        }
        if done {
            if !text.trim().is_empty() {
                self.import_text(&text);
            }
        } else {
            self.paste_box = Some(text);
        }
    }
}
