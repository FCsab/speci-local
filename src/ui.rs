//! What the specification editor and the structogram editor share: icons, colors, the
//! "unsaved changes" dialog and the status bar.

use eframe::egui::{self, Color32, Id, RichText};
use std::path::Path;

// Nerd Font icons (0xProto Nerd Font)
pub const I_NEW: &str = "\u{f016}";
pub const I_OPEN: &str = "\u{f07c}";
pub const I_SAVE: &str = "\u{f0c7}";
pub const I_RUN: &str = "\u{f04b}";
pub const I_OK: &str = "\u{f058}";
pub const I_FAIL: &str = "\u{f057}";
pub const I_ADD: &str = "\u{f067}";
pub const I_DEL: &str = "\u{f1f8}";
pub const I_WAND: &str = "\u{f0d0}";
pub const I_GEAR: &str = "\u{f013}";
pub const I_SPEC: &str = "\u{f121}";
pub const I_DATA: &str = "\u{f1c0}";
pub const I_VIEW: &str = "\u{f0ae}";
pub const I_WARN: &str = "\u{f071}";
pub const I_BULB: &str = "\u{f0eb}";
pub const I_COPY: &str = "\u{f0c5}";
pub const I_PASTE: &str = "\u{f0ea}";
pub const I_FIRST: &str = "\u{f049}";
pub const I_PREV: &str = "\u{f048}";
pub const I_NEXT: &str = "\u{f051}";
pub const I_LAST: &str = "\u{f050}";
pub const I_SYSTEM: &str = "\u{f0379}";
pub const I_DARK: &str = "\u{f0594}";
pub const I_LIGHT: &str = "\u{f0599}";
pub const I_STUKI: &str = "\u{f0e8}";
pub const I_UNDO: &str = "\u{f0e2}";
pub const I_REDO: &str = "\u{f01e}";
pub const I_IMAGE: &str = "\u{f03e}";
pub const I_DOWNLOAD: &str = "\u{f019}";
pub const I_HELP: &str = "\u{f059}";
pub const I_CLOSE: &str = "\u{f00d}";
pub const I_MOVE: &str = "\u{f047}";
pub const I_BRUSH: &str = "\u{f1fc}";
pub const I_SLIDERS: &str = "\u{f1de}";
pub const I_TEXT: &str = "\u{f0f6}";

pub const GREEN: Color32 = Color32::from_rgb(46, 160, 67);
pub const RED: Color32 = Color32::from_rgb(229, 72, 77);
pub const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);

/// Width for a dialog: `w`, or less on a narrow screen.
pub fn modal_width(ctx: &egui::Context, w: f32) -> f32 {
    w.min(ctx.content_rect().width() - 48.0).max(200.0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unsaved {
    Save,
    Discard,
    Cancel,
}

/// The "unsaved changes" dialog (in-app, so it works without zenity/kdialog on Linux).
pub fn unsaved_dialog(ctx: &egui::Context, id: &str, what: &str) -> Option<Unsaved> {
    let mut choice = None;
    let modal = egui::Modal::new(Id::unique(id)).show(ctx, |ui| {
        ui.set_width(modal_width(ctx, 420.0));
        ui.heading("Unsaved changes");
        ui.label(format!(
            "The current {what} has changes that are not saved yet."
        ));
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button(format!("{I_SAVE}  Save first")).clicked() {
                choice = Some(Unsaved::Save);
            }
            if ui.button("Discard changes").clicked() {
                choice = Some(Unsaved::Discard);
            }
            if ui.button("Cancel").clicked() {
                choice = Some(Unsaved::Cancel);
            }
        });
    });
    if modal.should_close() && choice.is_none() {
        choice = Some(Unsaved::Cancel); // Escape or a click outside
    }
    choice
}

pub fn file_stem(name: &str, fallback: &str) -> String {
    let name = name.trim();
    let name = if name.is_empty() { fallback } else { name };
    name.chars()
        .map(|c| if "/\\:*?\"<>|\n".contains(c) { '_' } else { c })
        .collect()
}

pub fn unsaved_dot(ui: &mut egui::Ui) {
    ui.label(RichText::new("●").color(ui.visuals().weak_text_color()))
        .on_hover_text("Unsaved changes");
}

pub fn status_bar(
    ui: &mut egui::Ui,
    dirty: bool,
    status: &str,
    path: Option<&Path>,
    what: &str,
    cursor: Option<(usize, usize)>,
    // Vim mode of the editors, when the Vim keybindings are on
    mode: Option<&str>,
) {
    let file = match path {
        Some(p) => p.display().to_string(),
        None => "not saved yet".into(),
    };
    ui.horizontal(|ui| {
        // right side first so the (possibly long) path can be truncated to the remaining width
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                    .small()
                    .weak(),
            );
            if let Some((l, c)) = cursor {
                ui.label(RichText::new(format!("Ln {l}, Col {c}")).small().weak());
            }
            if let Some(mode) = mode {
                ui.label(RichText::new(mode).small().monospace().color(ACCENT));
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if dirty {
                    ui.label(RichText::new("● modified").small().color(ACCENT));
                }
                if !status.is_empty() {
                    ui.add(egui::Label::new(RichText::new(status).small()).truncate());
                }
                ui.add(egui::Label::new(RichText::new(file).small().weak()).truncate())
                    .on_hover_text(format!("File of the current {what}"));
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stems() {
        assert_eq!(file_stem("  Páratlan számok ", "x"), "Páratlan számok");
        assert_eq!(
            file_stem("a/b\\c:d*e?f\"g<h>i|j\nk", "x"),
            "a_b_c_d_e_f_g_h_i_j_k"
        );
        assert_eq!(file_stem("   ", "specification"), "specification");
    }

    /// The app has no fonts but its own three: every character the code shows must be in them.
    #[test]
    fn every_character_of_the_interface_has_a_glyph() {
        fn sources(dir: &Path, out: &mut String) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    sources(&p, out);
                } else if !p.ends_with("tests.rs") {
                    out.push_str(&std::fs::read_to_string(&p).unwrap());
                }
            }
        }
        let mut text = String::new();
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut text,
        );
        let ctx = egui::Context::default();
        crate::app::fonts(&ctx);
        ctx.run_ui(Default::default(), |_| {})
            .textures_delta
            .clear();
        let mut missing: Vec<char> = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .flat_map(str::chars)
            // ⎋ stands for Escape in the Vim tests' key strings only
            .filter(|&c| c > '~' && !c.is_whitespace() && c != '⎋')
            .filter(|&c| {
                !ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::proportional(14.0), c))
                    || !ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::monospace(14.0), c))
            })
            .collect();
        missing.sort_unstable();
        missing.dedup();
        assert!(missing.is_empty(), "no glyph for {missing:?}");
        // a character none of the three fonts has is noticed (U+2715, once used for "delete")
        let cross = char::from_u32(0x2715).unwrap();
        assert!(!ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::proportional(14.0), cross)));
    }

    /// The embedded fonts only keep the icons named in this file (`scripts/subset-fonts.py`).
    #[test]
    fn every_icon_has_a_glyph_in_both_fonts() {
        let icons: Vec<char> = include_str!("ui.rs")
            .split("\\u{")
            .skip(1)
            .filter_map(|r| u32::from_str_radix(r.split('}').next()?, 16).ok())
            .filter_map(char::from_u32)
            .collect();
        assert!(icons.len() > 30, "{}", icons.len());
        for bytes in [
            &include_bytes!("../assets/0xProtoNerdFontPropo-Regular.ttf")[..],
            &include_bytes!("../assets/0xProtoNerdFontMono-Regular.ttf")[..],
        ] {
            let mut f = egui::FontDefinitions::empty();
            f.font_data.insert(
                "f".into(),
                std::sync::Arc::new(egui::FontData::from_static(bytes)),
            );
            f.families
                .insert(egui::FontFamily::Proportional, vec!["f".into()]);
            f.families
                .insert(egui::FontFamily::Monospace, vec!["f".into()]);
            let ctx = egui::Context::default();
            ctx.set_fonts(f);
            ctx.run_ui(Default::default(), |_| {})
                .textures_delta
                .clear();
            let font = egui::FontId::proportional(14.0);
            for &c in &icons {
                assert!(
                    ctx.fonts_mut(|f| f.has_glyph(&font, c)),
                    "no glyph for U+{:X}",
                    c as u32
                );
            }
            // a missing glyph is noticed
            assert!(!ctx.fonts_mut(|f| f.has_glyph(&font, char::from_u32(0xf2b9).unwrap())));
        }
    }
}
