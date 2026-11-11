//! Tells the user when a newer release is out on GitHub.

use super::App;
use crate::ui::modal_width;
use eframe::egui::{self, Id, OpenUrl};
use std::sync::mpsc::{Receiver, channel};

const RELEASES: &str = "https://github.com/FCsab/speci-local/releases/latest";
const API: &str = "https://api.github.com/repos/FCsab/speci-local/releases/latest";

/// Asks GitHub for the latest version in the background; the answer arrives on the channel.
pub(super) fn check(ctx: egui::Context) -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        if let Some(v) = latest() {
            let _ = tx.send(v);
            ctx.request_repaint();
        }
    });
    rx
}

// shortcut: uses the system's curl (Windows 10+, macOS, Linux; not Android) to avoid an HTTP/TLS
// dependency; without curl the check silently does nothing. Switch to `ureq` if that matters.
fn latest() -> Option<String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "10", API]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing up
    }
    let out = cmd.output().ok().filter(|o| o.status.success())?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(
        json["tag_name"]
            .as_str()?
            .trim_start_matches('v')
            .to_string(),
    )
}

/// `latest` is a higher version than `current` ("0.10.0" > "0.9.1").
fn newer(latest: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parts(latest) > parts(current)
}

impl App {
    pub(super) fn update_dialog(&mut self, ctx: &egui::Context) {
        if let Some(Ok(v)) = self.update_rx.as_ref().map(Receiver::try_recv) {
            self.update_rx = None;
            if newer(&v, env!("CARGO_PKG_VERSION")) && v != self.skipped_version {
                self.new_version = Some(v);
            }
        }
        let Some(v) = self.new_version.clone() else {
            return;
        };
        let mut done = false;
        let modal = egui::Modal::new(Id::unique("update-dialog")).show(ctx, |ui| {
            ui.set_width(modal_width(ctx, 420.0));
            ui.heading("Update available");
            ui.label(format!(
                "speci-local {v} is out. You have {}.",
                env!("CARGO_PKG_VERSION")
            ));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Open download page").clicked() {
                    ctx.open_url(OpenUrl::new_tab(RELEASES));
                    done = true;
                }
                if ui.button("Skip this version").clicked() {
                    self.skipped_version = v.clone();
                    done = true;
                }
                if ui.button("Don't show again").clicked() {
                    self.no_update_check = true;
                    done = true;
                }
            });
        });
        if done || modal.should_close() {
            self.new_version = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::newer;

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.6.0", "0.5.0"));
        assert!(newer("0.10.0", "0.9.1"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.5.0", "0.5.0"));
        assert!(!newer("0.4.9", "0.5.0"));
    }
}
