//! The toolbar at the top and the status bar at the bottom.

use super::theme::theme_buttons;
use super::{App, Tab, Tool, data_editor_id, line_col, spec_editor_id};
use crate::editor::cursor_of;
use crate::platform;
use crate::ui::{
    self, ACCENT, GREEN, I_COPY, I_DATA, I_FAIL, I_GEAR, I_NEW, I_OK, I_OPEN, I_PASTE, I_SAVE,
    I_SPEC, I_STUKI, I_VIEW, I_WARN, RED,
};
use eframe::egui::{self, RichText, TextEdit, TextStyle};

/// Toolbar actions on the current file.
#[derive(Clone, Copy)]
enum Act {
    New,
    Open,
    Copy,
    Paste,
    Save,
}

impl App {
    fn act(&mut self, act: Act, ui: &egui::Ui) {
        let shift = ui.input(|i| i.modifiers.shift);
        match act {
            Act::New => self.file_new(),
            Act::Open => self.file_open(),
            // structograms: a picture to paste into documents; Shift-click: the project as a text line
            Act::Copy if self.tool == Tool::Stuki && !shift => {
                self.stuki.copy_image(ui.ctx(), None)
            }
            Act::Copy if self.tool == Tool::Stuki => self.stuki.copy_project(ui.ctx()),
            Act::Copy => self.copy_project(ui.ctx()),
            Act::Paste => self.paste_project(),
            Act::Save => self.file_save(shift),
        }
    }

    /// New, Open | Copy, Paste, Save — reversed for right-to-left layouts.
    fn file_buttons(&mut self, ui: &mut egui::Ui, rtl: bool) {
        let mut groups = [
            vec![Act::New, Act::Open],
            vec![Act::Copy, Act::Paste, Act::Save],
        ];
        if rtl {
            groups.reverse();
            groups.iter_mut().for_each(|g| g.reverse());
        }
        for (n, group) in groups.iter().enumerate() {
            if n > 0 {
                ui.separator();
            }
            for &act in group {
                let stuki = self.tool == Tool::Stuki;
                let (icon, tip) = match act {
                    Act::New if stuki => (I_NEW, "New structogram (Ctrl+N)"),
                    Act::Open if stuki => (I_OPEN, "Open a .st file (Ctrl+O)"),
                    Act::New => (I_NEW, "New specification (Ctrl+N)"),
                    Act::Open => (I_OPEN, "Open a .sp file (Ctrl+O)"),
                    Act::Copy if stuki => (
                        I_COPY,
                        "Copy the selected structogram as a picture (Ctrl+C)\nShift-click: copy the whole project as one text line (Ctrl+Shift+C)",
                    ),
                    Act::Copy => (
                        I_COPY,
                        "Copy the whole project to the clipboard as one compact line (Ctrl+Shift+C)",
                    ),
                    Act::Paste => (I_PASTE, "Read a project from the clipboard (Ctrl+Shift+V)"),
                    Act::Save if stuki => (
                        I_SAVE,
                        "Save as .st (Ctrl+S)\nShift-click: Save as… (Ctrl+Shift+S)",
                    ),
                    Act::Save => (
                        I_SAVE,
                        "Save (Ctrl+S)\nShift-click: Save as… (Ctrl+Shift+S)",
                    ),
                };
                if ui
                    .button(icon)
                    .on_hover_text(platform::shortcut(tip))
                    .clicked()
                {
                    self.act(act, ui);
                }
            }
        }
    }

    fn settings_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(I_GEAR, |ui| {
            if ui.button(format!("{I_SAVE}  Save as…")).clicked() {
                self.file_save(true);
                ui.close();
            }
            ui.separator();
            ui.label("Font size");
            ui.add(egui::Slider::new(&mut self.font, 12.0..=28.0).suffix(" px"));
            ui.separator();
            if ui.checkbox(&mut self.vim, "Vim keybindings").changed() {
                self.vim_state.reset();
            }
            if self.narrow {
                ui.separator();
                ui.label("Theme");
                ui.horizontal(|ui| theme_buttons(ui, false));
            }
        })
        .response
        .on_hover_text("Settings");
    }

    /// Switches the Vim keybindings of the code editors on and off.
    fn vim_toggle(&mut self, ui: &mut egui::Ui) {
        let label = RichText::new("Vim").monospace();
        if ui
            .toggle_value(&mut self.vim, label)
            .on_hover_text(
                "Vim keybindings in the editors\nEsc: Normal mode · i/a/o: Insert mode · v/V: Visual mode",
            )
            .changed()
        {
            self.vim_state.reset();
        }
    }

    /// Specification / Stuki tabs.
    fn tool_tabs(&mut self, ui: &mut egui::Ui) {
        let short = self.narrow;
        ui.selectable_value(
            &mut self.tool,
            Tool::Spec,
            if short {
                format!("{I_SPEC} Spec")
            } else {
                format!("{I_SPEC}  Specification")
            },
        )
        .on_hover_text("Specification editor with test cases (.sp)");
        ui.selectable_value(&mut self.tool, Tool::Stuki, format!("{I_STUKI}  Stuki"))
            .on_hover_text("Structogram editor (.st)");
    }

    /// The task title or the file name, at most `width` wide, then the unsaved-changes dot.
    fn title(&mut self, ui: &mut egui::Ui, width: f32) {
        // room for the dot, so it stays clear of what comes after
        let width = width.min(ui.available_width() - 2.0 * self.font).max(0.0);
        let dirty = match self.tool {
            Tool::Stuki => {
                ui.scope(|ui| {
                    ui.set_max_width(width);
                    ui.add(
                        egui::Label::new(RichText::new(self.stuki.file_name()).strong()).truncate(),
                    )
                    .on_hover_text("File of the current structogram");
                });
                self.stuki.dirty()
            }
            Tool::Spec => {
                ui.add(
                    TextEdit::singleline(&mut self.doc.title)
                        .hint_text("Untitled task")
                        .font(TextStyle::Heading)
                        .frame(egui::Frame::NONE)
                        .desired_width(width),
                );
                self.dirty()
            }
        };
        if dirty {
            ui::unsaved_dot(ui);
        }
    }

    pub(super) fn verdicts(&self) -> (usize, usize) {
        let done = self.results.iter().flatten();
        let passed = done.clone().filter(|o| o.pass).count();
        (passed, done.count() - passed)
    }

    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let narrow = self.narrow;
        let mut one_row = true;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("speci")
                    .strong()
                    .size(self.font + 2.0)
                    .color(ACCENT),
            );
            ui.label(RichText::new("local").size(self.font + 2.0).weak());
            ui.add_space(if narrow { 4.0 } else { 12.0 });
            self.tool_tabs(ui);
            if narrow {
                return;
            }
            ui.separator();
            // the buttons go on a row of their own when they would cover the tabs (issue #8)
            one_row = ui.available_width() >= self.font * 6.0 + self.toolbar_right;
            if !one_row {
                self.title(ui, 260.0);
                return;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.right_buttons(ui);
                // the title gets what is left
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    self.title(ui, 260.0)
                });
            });
        });
        if narrow {
            self.narrow_rows(ui);
        } else if !one_row {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.right_buttons(ui)
                })
            });
        }
    }

    /// Files, panes, Vim, settings and theme, laid out right to left.
    fn right_buttons(&mut self, ui: &mut egui::Ui) {
        let right = ui.max_rect().right();
        theme_buttons(ui, true);
        self.settings_menu(ui);
        ui.separator();
        if self.tool == Tool::Spec {
            self.vim_toggle(ui);
            ui.separator();
            ui.toggle_value(&mut self.show_v, I_VIEW)
                .on_hover_text("Evaluation pane");
            ui.toggle_value(&mut self.show_d, I_DATA)
                .on_hover_text("Test cases pane");
            ui.toggle_value(&mut self.show_s, I_SPEC)
                .on_hover_text("Specification pane");
            ui.separator();
        }
        self.file_buttons(ui, true);
        // changes with the font size and the tool: lay this frame out again with the new width
        let width = right - ui.min_rect().left();
        if (width - self.toolbar_right).abs() > 0.5 {
            self.toolbar_right = width;
            ui.ctx().request_discard("toolbar width changed");
        }
    }

    /// Phones: the title, then the buttons, then one tab per pane.
    fn narrow_rows(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 24.0).max(60.0);
            self.title(ui, width);
        });
        ui.horizontal_wrapped(|ui| {
            self.file_buttons(ui, false);
            ui.separator();
            self.settings_menu(ui);
        });
        if self.tool == Tool::Stuki {
            return;
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Spec, "Spec");
            ui.selectable_value(&mut self.tab, Tab::Data, "Tests");
            let (ok, bad) = self.verdicts();
            let font = TextStyle::Button.resolve(ui.style());
            let text = if self.tab == Tab::Eval {
                ui.visuals().selection.stroke.color
            } else {
                ui.visuals().widgets.inactive.text_color()
            };
            let mut label = egui::text::LayoutJob::default();
            let mut part = |s: &str, color| {
                label.append(s, 0.0, egui::TextFormat::simple(font.clone(), color));
            };
            part("Results", text);
            if !self.spec_diags.is_empty() {
                part(&format!(" {I_WARN}"), RED);
            }
            if ok > 0 {
                part(&format!("  {I_OK} {ok}"), GREEN);
            }
            if bad > 0 {
                part(&format!("  {I_FAIL} {bad}"), RED);
            }
            ui.selectable_value(&mut self.tab, Tab::Eval, label);
        });
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        if self.tool == Tool::Stuki {
            let st = &self.stuki;
            ui::status_bar(
                ui,
                st.dirty(),
                &st.status,
                st.path.as_deref(),
                "structogram",
                None,
                None,
            );
            return;
        }
        let cursor = [
            (spec_editor_id(), Some(&self.doc.spec)),
            (
                data_editor_id(self.current),
                self.doc.tests.get(self.current),
            ),
        ]
        .into_iter()
        .find(|(id, _)| ui.memory(|m| m.has_focus(*id)))
        .and_then(|(id, text)| Some(line_col(text?, cursor_of(ui.ctx(), id)?.1)));
        let mode = self.vim.then(|| self.vim_state.status());
        ui::status_bar(
            ui,
            self.dirty(),
            &self.status,
            self.path.as_deref(),
            "specification",
            cursor,
            mode.as_deref(),
        );
    }
}
