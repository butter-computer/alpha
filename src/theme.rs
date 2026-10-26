//! Calm, low-contrast light and dark palettes.

use gpui::{Global, Hsla, rgb, rgba};

#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub bg: Hsla,
    pub fg: Hsla,
    /// Markdown syntax markers on the active line.
    pub marker: Hsla,
    /// Markdown syntax markers elsewhere.
    pub marker_faint: Hsla,
    /// Placeholder, toolbar text, secondary labels.
    pub muted: Hsla,
    pub accent: Hsla,
    pub code_bg: Hsla,
    pub quote_bar: Hsla,
    pub hairline: Hsla,
    pub selection: Hsla,
    pub find: Hsla,
    pub find_current: Hsla,
    pub panel: Hsla,
    pub hover: Hsla,
}

impl Global for Theme {}

impl Theme {
    pub fn light() -> Self {
        Self {
            dark: false,
            bg: rgb(0xFBFAF7).into(),
            fg: rgb(0x2A2723).into(),
            marker: rgba(0x2A272366).into(),
            marker_faint: rgba(0x2A272338).into(),
            muted: rgb(0x9A948A).into(),
            accent: rgb(0x9A5530).into(),
            code_bg: rgba(0x2A27230C).into(),
            quote_bar: rgba(0x2A272330).into(),
            hairline: rgba(0x2A272316).into(),
            selection: rgba(0x9A553033).into(),
            find: rgba(0xE8B93B55).into(),
            find_current: rgba(0xE89A3BAA).into(),
            panel: rgb(0xFFFFFF).into(),
            hover: rgba(0x2A272310).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: rgb(0x1C1B1A).into(),
            fg: rgb(0xE2DED8).into(),
            marker: rgba(0xE2DED87A).into(),
            marker_faint: rgba(0xE2DED84A).into(),
            muted: rgb(0x7D7870).into(),
            accent: rgb(0xDCA680).into(),
            code_bg: rgba(0xFFFFFF10).into(),
            quote_bar: rgba(0xE2DED840).into(),
            hairline: rgba(0xFFFFFF14).into(),
            selection: rgba(0xDCA68040).into(),
            find: rgba(0xE8B93B44).into(),
            find_current: rgba(0xE89A3B99).into(),
            panel: rgb(0x262524).into(),
            hover: rgba(0xFFFFFF12).into(),
        }
    }
}
