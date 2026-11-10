//! How structograms look: the user's appearance settings and the theme's colors.

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 16.0..=64.0;

/// The web app's "Appearance" and "Advanced settings".
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
    /// Font size of the diagrams in points ("Zoom", 16–64).
    pub zoom: f32,
    /// Border width in em (0.1–0.2).
    pub border: f32,
    /// Horizontal / vertical distance inside branches, in 0.1 em.
    pub hdist: u32,
    pub vdist: u32,
    /// Change the two distances together.
    pub linked: bool,
    /// No distance between a condition and the branch below it.
    pub snapping: bool,
    /// Extra height of the blocks, in 0.025 em.
    pub block_height: u32,
    /// Space around the elements, in 0.1 em.
    pub spacing: u32,
    /// Space right of a function's body.
    pub right_padding: bool,
    /// Color the elements by kind.
    pub colorized: bool,
    pub proportional: bool,
    /// Copied / exported pictures always use the light look (white background, black lines),
    /// whatever the app's theme.
    pub light_pictures: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            zoom: 16.0,
            border: 0.125,
            hdist: 0,
            vdist: 0,
            linked: true,
            snapping: false,
            block_height: 0,
            spacing: 0,
            right_padding: false,
            colorized: true,
            proportional: false,
            light_pictures: true,
        }
    }
}

/// Colors that follow the light / dark theme (the web app's `--struct-*` variables).
#[derive(Clone, Copy)]
pub struct Look {
    pub border: Color32,
    pub text: Color32,
    pub bg: Color32,
}

impl Look {
    pub fn new(dark: bool) -> Look {
        if dark {
            // rgba(255,255,255,0.7) over #555, made opaque so shared lines don't get brighter
            let c = Color32::from_gray(0xcc);
            Look {
                border: c,
                text: c,
                bg: Color32::from_gray(0x55),
            }
        } else {
            Look {
                border: Color32::BLACK,
                text: Color32::BLACK,
                bg: Color32::WHITE,
            }
        }
    }
}
