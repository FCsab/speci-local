//! Starting the app on the desktop systems and on Android.

use super::App;
use super::theme::fonts;
use crate::platform;
use std::path::PathBuf;

#[cfg(not(target_os = "android"))]
const APP_ID: &str = "speci-local";
#[cfg(not(target_os = "android"))]
const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");

/// Wayland compositors (KDE, GNOME, …) take a window's icon from the installed `.desktop` file whose
/// name matches the app id, not from the window itself. Install one (plus the icon) for the current
/// user, so the window, taskbar and launcher show the real icon. Only rewritten when it changed.
#[cfg(target_os = "linux")]
fn install_desktop_entry() -> std::io::Result<()> {
    let data = match std::env::var_os("XDG_DATA_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME").ok_or(std::io::ErrorKind::NotFound)?)
            .join(".local/share"),
    };
    let icon = data
        .join("icons/hicolor/256x256/apps")
        .join(format!("{APP_ID}.png"));
    if std::fs::read(&icon).ok().as_deref() != Some(ICON_PNG) {
        std::fs::create_dir_all(icon.parent().unwrap())?;
        std::fs::write(&icon, ICON_PNG)?;
    }
    let exe = std::env::current_exe()?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=speci-local\nComment=Offline ELTE specification editor\nExec=\"{}\" %f\nIcon={APP_ID}\nStartupWMClass={APP_ID}\nTerminal=false\nCategories=Development;Education;\n",
        exe.display()
    );
    let desktop = data.join("applications").join(format!("{APP_ID}.desktop"));
    if std::fs::read_to_string(&desktop).ok().as_deref() != Some(entry.as_str()) {
        std::fs::create_dir_all(desktop.parent().unwrap())?;
        std::fs::write(&desktop, entry)?;
    }
    Ok(())
}

/// Load the fonts and the saved session, then open `file` if given.
pub(super) fn create(cc: &eframe::CreationContext, file: Option<PathBuf>) -> App {
    fonts(&cc.egui_ctx);
    let mut app: App = cc
        .storage
        .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
        .unwrap_or_else(|| App {
            // phone screens are narrow: start a bit smaller than on the desktop
            font: if platform::MOBILE { 15.0 } else { 18.0 },
            ..App::default()
        });
    if !app.no_update_check {
        app.update_rx = Some(super::update::check(cc.egui_ctx.clone()));
    }
    // `speci-local file.sp` (also how double-clicking a .sp file opens it)
    if let Some(p) = file {
        app.open_path(p);
    }
    app
}

/// Windows, Linux and macOS.
#[cfg(not(target_os = "android"))]
pub fn run_desktop(file: Option<PathBuf>) -> eframe::Result {
    #[cfg(target_os = "linux")]
    if let Err(e) = install_desktop_entry() {
        eprintln!("speci-local: could not install the desktop entry (window icon): {e}");
    }
    let mut viewport = eframe::egui::ViewportBuilder::default().with_app_id(APP_ID);
    // X11 and Windows use this directly (macOS takes the icon from the app bundle)
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport: viewport
            .with_inner_size([1360.0, 900.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("speci-local"),
        ..Default::default()
    };
    eframe::run_native(
        "speci-local",
        options,
        Box::new(move |cc| Ok(Box::new(create(cc, file)))),
    )
}

/// Android: called from the NativeActivity's `android_main`.
#[cfg(target_os = "android")]
pub fn run_android(android: platform::android::AndroidApp) -> eframe::Result {
    platform::android::init(&android);
    let options = eframe::NativeOptions {
        persistence_path: android.internal_data_path().map(|d| d.join("app.ron")),
        android_app: Some(android),
        ..Default::default()
    };
    eframe::run_native(
        "speci-local",
        options,
        Box::new(|cc| Ok(Box::new(create(cc, None)))),
    )
}
