#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")] // no console window on Windows

/// Windows, Linux and macOS. `speci-local file.sp` opens the file.
#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    speci_local::app::run_desktop(std::env::args_os().nth(1).map(Into::into))
}

/// The Android app is the `android/` crate: a library that the activity loads.
#[cfg(target_os = "android")]
fn main() {}
