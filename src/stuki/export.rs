//! Structograms as pictures: copied to the clipboard or exported as PNG files.

use super::layout::{Lay, css_color};
use super::settings::Look;
use super::{Stuki, png};
use crate::platform;
use eframe::egui::{self, Vec2, pos2};
use std::path::Path;

impl Stuki {
    /// Index of the structogram the PNG buttons and the background colors work on.
    pub(super) fn current_diagram(&self) -> usize {
        self.selected
            .and_then(|s| self.program.diagram_of(s))
            .or_else(|| {
                let d = self.focus_diagram?;
                self.program.diagrams.iter().position(|x| x.id == d)
            })
            .unwrap_or(0)
    }

    /// Structogram `index` (default: the selected one) as a picture.
    pub(crate) fn render_png(
        &self,
        ctx: &egui::Context,
        index: Option<usize>,
    ) -> Option<egui::ColorImage> {
        let d = self
            .program
            .diagrams
            .get(index.unwrap_or_else(|| self.current_diagram()))?;
        let ppp = ctx.pixels_per_point();
        // at least twice the font size in pixels, so the picture stays sharp when scaled
        let scale = (2.0 / ppp).max(1.0);
        let dark = ctx.global_style().visuals.dark_mode && !self.settings.light_pictures;
        let look = Look::new(dark);
        let settings = self.settings.clone();
        let mut lay = Lay::new(ctx, &settings, look, scale);
        lay.hints = false;
        let size = lay.diagram_size(d);
        let margin = (settings.border * lay.em()).max(1.0);
        lay.place_diagram(d, pos2(margin, margin));
        let bg = d.bg.as_deref().and_then(css_color).unwrap_or(look.bg);
        Some(png::render(
            ctx,
            lay.out.shapes(true),
            size + Vec2::splat(2.0 * margin),
            ppp,
            bg,
        ))
    }

    /// Copy structogram `index` (default: the selected one) to the clipboard as a picture.
    pub fn copy_image(&mut self, ctx: &egui::Context, index: Option<usize>) {
        let Some(img) = self.render_png(ctx, index) else {
            return;
        };
        if platform::MOBILE {
            // Android's clipboard only takes images as files shared by a content provider
            self.export_to_folder(&img, "Android can't copy pictures to the clipboard: saved");
            return;
        }
        self.status = format!(
            "Copied the structogram as a {}×{} picture: paste it with {}",
            img.size[0],
            img.size[1],
            platform::shortcut("Ctrl+V")
        );
        ctx.copy_image(img);
    }

    pub(super) fn export_png(&mut self, ctx: &egui::Context, index: Option<usize>) {
        let Some(img) = self.render_png(ctx, index) else {
            return;
        };
        #[cfg(not(target_os = "android"))]
        {
            let name = self.suggested_name();
            let png = platform::FileKind::Png;
            if let Some(path) = platform::pick_save(&name, png) {
                self.write_png(&img, &png.with_ext(path), "Exported");
            }
        }
        #[cfg(target_os = "android")]
        self.export_to_folder(&img, "Exported");
    }

    fn export_to_folder(&mut self, img: &egui::ColorImage, what: &str) {
        match platform::documents_dir() {
            Some(dir) => {
                let path = dir.join(format!("{}.png", self.suggested_name()));
                self.write_png(img, &path, what);
            }
            None => self.status = "There is no folder to save the picture in".into(),
        }
    }

    fn write_png(&mut self, img: &egui::ColorImage, path: &Path, what: &str) {
        self.status = match png::encode(img)
            .and_then(|b| platform::write_file(path, &b).map_err(|e| e.to_string()))
        {
            Ok(()) => format!("{what} {}", path.display()),
            Err(e) => format!("Could not save {}: {e}", path.display()),
        };
    }
}
