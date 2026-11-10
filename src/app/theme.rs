//! Fonts, colors and the small building blocks every pane uses.

use crate::ui::{ACCENT, I_DARK, I_LIGHT, I_SYSTEM};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke, TextStyle, Theme};

pub(crate) fn fonts(ctx: &egui::Context) {
    let mut f = egui::FontDefinitions::default();
    let add = |f: &mut egui::FontDefinitions, name: &str, bytes: &'static [u8]| {
        f.font_data.insert(
            name.into(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    };
    add(
        &mut f,
        "proto",
        include_bytes!("../../assets/0xProtoNerdFontPropo-Regular.ttf"),
    );
    add(
        &mut f,
        "proto-mono",
        include_bytes!("../../assets/0xProtoNerdFontMono-Regular.ttf"),
    );
    add(
        &mut f,
        "math",
        include_bytes!("../../assets/NotoSansMath-Regular.ttf"),
    );
    // 0xProto lacks ∈ ∀ ∃ →, so Noto Sans Math comes right after it
    for (fam, main) in [
        (egui::FontFamily::Proportional, "proto"),
        (egui::FontFamily::Monospace, "proto-mono"),
    ] {
        let list = f.families.entry(fam).or_default();
        list.insert(0, main.into());
        list.insert(1, "math".into());
    }
    ctx.set_fonts(f);
}

pub(super) fn theme(ctx: &egui::Context, font: f32) {
    ctx.all_styles_mut(|s| {
        for (ts, f) in s.text_styles.iter_mut() {
            f.size = match ts {
                TextStyle::Heading => font + 4.0,
                TextStyle::Small => (font * 0.72).max(10.0),
                TextStyle::Button => font * 0.9,
                _ => font,
            };
        }
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = font + 10.0;
        let r = CornerRadius::same(6);
        for w in [
            &mut s.visuals.widgets.noninteractive,
            &mut s.visuals.widgets.inactive,
            &mut s.visuals.widgets.hovered,
            &mut s.visuals.widgets.active,
            &mut s.visuals.widgets.open,
        ] {
            w.corner_radius = r;
        }
        s.visuals.window_corner_radius = CornerRadius::same(10);
        s.visuals.menu_corner_radius = CornerRadius::same(8);
        s.visuals.selection.bg_fill = ACCENT.linear_multiply(0.55);
        s.visuals.hyperlink_color = ACCENT;
    });
    ctx.style_mut_of(Theme::Dark, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_rgb(0x15, 0x17, 0x1c);
        v.window_fill = Color32::from_rgb(0x1d, 0x20, 0x27);
        v.extreme_bg_color = Color32::from_rgb(0x1b, 0x1e, 0x24);
        v.faint_bg_color = Color32::from_rgb(0x22, 0x26, 0x2e);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x26, 0x2a, 0x33);
        v.widgets.inactive.bg_fill = Color32::from_rgb(0x26, 0x2a, 0x33);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x30, 0x35, 0x40);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2e, 0x37));
    });
    ctx.style_mut_of(Theme::Light, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_rgb(0xf3, 0xf4, 0xf6);
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(0xea, 0xec, 0xf0);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0xe4, 0xe7, 0xeb);
        v.widgets.inactive.bg_fill = Color32::from_rgb(0xe4, 0xe7, 0xeb);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xd8, 0xdc, 0xe2);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xdc, 0xdf, 0xe4));
    });
}

/// Rounded "card" holding an editor or results.
pub(super) fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(4))
}

pub(super) fn pane_title(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.label(
        RichText::new(format!("{icon}  {title}"))
            .strong()
            .color(ui.visuals().strong_text_color()),
    );
}

/// System / dark / light, drawn with the bundled icon font (egui's own buttons use emoji,
/// which need a system emoji font that Android doesn't give egui).
pub(super) fn theme_buttons(ui: &mut egui::Ui, rtl: bool) {
    let mut pref = ui.ctx().options(|o| o.theme_preference);
    let mut items = [
        (
            egui::ThemePreference::System,
            I_SYSTEM,
            "Follow the system theme",
        ),
        (egui::ThemePreference::Dark, I_DARK, "Dark theme"),
        (egui::ThemePreference::Light, I_LIGHT, "Light theme"),
    ];
    if rtl {
        items.reverse();
    }
    for (p, icon, tip) in items {
        if ui
            .selectable_value(&mut pref, p, icon)
            .on_hover_text(tip)
            .changed()
        {
            ui.ctx().set_theme(pref);
        }
    }
}
