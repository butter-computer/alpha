//! App side of the progressive edge blur (the blur itself lives in the patched
//! gpui Metal renderer; see `vendor/README.md`).
//!
//! Everything painted before [`BlurSplit`] is blurred inside the top and bottom
//! bands; everything painted after it (toolbar, popups) stays sharp on top.

use crate::theme::Theme;
use crate::toolbar;
use gpui::{
    App, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Position, Style, Window,
};

/// Top band height in points (includes the titlebar area).
pub const TOP: f32 = 56.0;
/// Bottom band: toolbar height (solid) plus this much (the blur/fade ramp above the toolbar).
pub const BOTTOM_EXTRA: f32 = 40.0;
/// Blur radius at the window edge, in points (ramps from 0 at the inner edge of each band).
const MAX_RADIUS: f32 = 6.0;
/// Opacity of the window-background tint at the very window edge: 0 at the inner edge of each
/// band, easing smoothly to this value (1.0 = fully the background colour).
const EDGE_FADE: f32 = 1.0;

/// Enables the edge blur with a tint matching the theme background.
pub fn configure(theme: &Theme) {
    let bg = gpui::Rgba::from(theme.bg);
    gpui::set_edge_blur(Some(gpui::EdgeBlurConfig {
        top: TOP,
        bottom: toolbar::HEIGHT + BOTTOM_EXTRA,
        // The toolbar row itself is solid background; the ramp happens in the band above it.
        bottom_solid: toolbar::HEIGHT,
        max_radius: MAX_RADIUS,
        tint: [bg.r, bg.g, bg.b],
        tint_opacity: EDGE_FADE,
    }));
}

/// Zero-size element that marks the blur split point in paint order.
pub struct BlurSplit;

impl IntoElement for BlurSplit {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for BlurSplit {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.position = Position::Absolute;
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: gpui::Bounds<gpui::Pixels>, _: &mut (), _: &mut Window, _: &mut App) {}
    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: gpui::Bounds<gpui::Pixels>, _: &mut (), _: &mut (), window: &mut Window, _: &mut App) {
        window.paint_edge_blur_split();
    }
}
