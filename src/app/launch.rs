//! Starting the app on the desktop systems and on Android.

use super::App;
use super::theme::fonts;
use crate::platform;
use std::path::PathBuf;

#[cfg(not(target_os = "android"))]
const APP_ID: &str = "speci-local";
#[cfg(not(target_os = "android"))]
const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");

#[cfg(target_os = "linux")]
fn data_dir() -> std::io::Result<PathBuf> {
    Ok(match std::env::var_os("XDG_DATA_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME").ok_or(std::io::ErrorKind::NotFound)?)
            .join(".local/share"),
    })
}

#[cfg(target_os = "linux")]
fn desktop_file() -> std::io::Result<PathBuf> {
    Ok(data_dir()?
        .join("applications")
        .join(format!("{APP_ID}.desktop")))
}

/// The app has a visible entry in the app menu (Linux: the `.desktop` file without NoDisplay).
#[cfg(target_os = "linux")]
fn in_menu() -> bool {
    desktop_file()
        .and_then(std::fs::read_to_string)
        .is_ok_and(|e| !e.contains("NoDisplay=true"))
}

/// Wayland compositors (KDE, GNOME, …) take a window's icon from the installed `.desktop` file whose
/// name matches the app id, not from the window itself. Install one (plus the icon) for the current
/// user, so the window, taskbar and launcher show the real icon; it stays out of the app menu
/// (NoDisplay) unless `show`. Only rewritten when it changed.
#[cfg(target_os = "linux")]
fn install_desktop_entry(show: bool) -> std::io::Result<()> {
    let data = data_dir()?;
    let icon = data
        .join("icons/hicolor/256x256/apps")
        .join(format!("{APP_ID}.png"));
    if std::fs::read(&icon).ok().as_deref() != Some(ICON_PNG) {
        std::fs::create_dir_all(icon.parent().unwrap())?;
        std::fs::write(&icon, ICON_PNG)?;
    }
    let exe = std::env::current_exe()?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=speci-local\nComment=Offline ELTE specification editor\nExec=\"{}\" %f\nIcon={APP_ID}\nStartupWMClass={APP_ID}\nTerminal=false\nCategories=Development;Education;\n{}",
        exe.display(),
        if show { "" } else { "NoDisplay=true\n" }
    );
    let desktop = desktop_file()?;
    if std::fs::read_to_string(&desktop).ok().as_deref() != Some(entry.as_str()) {
        std::fs::create_dir_all(desktop.parent().unwrap())?;
        std::fs::write(&desktop, entry)?;
    }
    Ok(())
}

#[cfg(windows)]
fn start_menu_link() -> Option<PathBuf> {
    Some(
        PathBuf::from(std::env::var_os("APPDATA")?)
            .join(r"Microsoft\Windows\Start Menu\Programs\speci-local.lnk"),
    )
}

#[cfg(windows)]
fn in_menu() -> bool {
    start_menu_link().is_some_and(|l| l.exists())
}

/// A Start menu shortcut to this .exe, made by PowerShell (no extra dependency for a .lnk writer).
/// shortcut: points at where the .exe is now; moving the .exe breaks it until it is made again.
#[cfg(windows)]
fn add_to_menu() -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    let link = start_menu_link().ok_or(std::io::ErrorKind::NotFound)?;
    let exe = std::env::current_exe()?;
    // the paths go in as environment variables, so quotes in them can't break the script
    let ok = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:SL_LINK); \
             $s.TargetPath = $env:SL_EXE; $s.WorkingDirectory = Split-Path $env:SL_EXE; $s.Save()",
        ])
        .env("SL_LINK", &link)
        .env("SL_EXE", &exe)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: no console flashing up
        .status()?
        .success();
    if ok {
        Ok(())
    } else {
        Err(std::io::Error::other(
            "PowerShell could not make the shortcut",
        ))
    }
}

#[cfg(target_os = "linux")]
fn add_to_menu() -> std::io::Result<()> {
    install_desktop_entry(true)
}

/// First launch: offer to add the app to the app menu (Linux) or Start menu (Windows), once.
/// macOS: the .app goes wherever the user drags it, so there is nothing to ask.
fn offer_menu_entry(app: &mut App) {
    if app.menu_asked || cfg!(test) {
        return;
    }
    app.menu_asked = true;
    #[cfg(any(target_os = "linux", windows))]
    if !in_menu()
        && rfd::MessageDialog::new()
            .set_title("speci-local")
            .set_description("Add speci-local to the app menu?")
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes
        && let Err(e) = add_to_menu()
    {
        eprintln!("speci-local: could not add the app to the menu: {e}");
    }
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
    // tests must not hit the network: a fresh release would pop a modal over the UI under test
    if !app.no_update_check && !cfg!(test) {
        app.update_rx = Some(super::update::check(cc.egui_ctx.clone()));
    }
    offer_menu_entry(&mut app);
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
    // keep the user's choice: visible in the menu only if it already was
    if let Err(e) = install_desktop_entry(in_menu()) {
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
