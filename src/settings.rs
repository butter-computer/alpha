//! User settings, persisted as JSON in the platform config directory.

use gpui::Global;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MIN_FONT_SIZE: f32 = 12.0;
pub const MAX_FONT_SIZE: f32 = 32.0;
pub const DEFAULT_FONT_SIZE: f32 = 16.0;

pub use crate::fonts::SERIF_FAMILY;
pub const SANS_FAMILY: &str = "Helvetica Neue";
pub const MONO_FAMILY: &str = "Menlo";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontChoice {
    #[default]
    Serif,
    Sans,
    Mono,
}

impl FontChoice {
    pub fn family(self) -> &'static str {
        match self {
            FontChoice::Serif => SERIF_FAMILY,
            FontChoice::Sans => SANS_FAMILY,
            FontChoice::Mono => MONO_FAMILY,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub font_size: f32,
    pub font: FontChoice,
}

impl Global for Settings {}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: ThemeMode::System, font_size: DEFAULT_FONT_SIZE, font: FontChoice::Serif }
    }
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("alpha").join("settings.json"))
    }

    /// Parses settings, falling back to defaults on any error, and clamps values.
    pub fn from_json(s: &str) -> Self {
        let mut v: Settings = serde_json::from_str(s).unwrap_or_default();
        v.sanitize();
        v
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| Self::from_json(&s))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn sanitize(&mut self) {
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
        } else {
            DEFAULT_FONT_SIZE
        };
    }

    pub fn adjust_font_size(&mut self, delta: f32) {
        self.font_size += delta;
        self.sanitize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_serif_system() {
        let s = Settings::default();
        assert_eq!(s.theme, ThemeMode::System);
        assert_eq!(s.font, FontChoice::Serif);
    }

    #[test]
    fn roundtrip() {
        let s = Settings { theme: ThemeMode::Dark, font_size: 22.0, font: FontChoice::Mono };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(Settings::from_json(&json), s);
    }

    #[test]
    fn bad_input_falls_back_and_clamps() {
        assert_eq!(Settings::from_json("not json"), Settings::default());
        assert_eq!(Settings::from_json("{\"font_size\": 500}").font_size, MAX_FONT_SIZE);
        assert_eq!(Settings::from_json("{\"theme\":\"light\"}").theme, ThemeMode::Light);
    }

    #[test]
    fn font_size_adjust_is_bounded() {
        let mut s = Settings::default();
        for _ in 0..100 {
            s.adjust_font_size(1.0);
        }
        assert_eq!(s.font_size, MAX_FONT_SIZE);
        for _ in 0..100 {
            s.adjust_font_size(-1.0);
        }
        assert_eq!(s.font_size, MIN_FONT_SIZE);
    }
}
