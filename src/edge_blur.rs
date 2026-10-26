//! Native scroll-edge blur for the top and bottom of the window (macOS).
//!
//! Two `NSVisualEffectView`s (blending mode `withinWindow`) are added above
//! GPUI's content view, each with a gradient layer mask so the blur and a
//! background-coloured tint fade in progressively toward the window edge.
//! The tint means that, even if a blur did not sample the Metal content, text
//! still fades out instead of being hard-clipped.
//!
//! Limitations: native views sit above *everything* GPUI draws, so GPUI UI
//! placed inside a band would be blurred too. The bottom band therefore ends
//! at the top of the (solid) toolbar, and overlays are positioned clear of both bands.

use crate::theme::Theme;
use gpui::Window;

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)] // the `objc` macros probe a cfg that this crate does not define
mod mac {
    use super::*;
    use cocoa::base::{BOOL, NO, YES, id, nil};
    use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::sync::Once;

    #[link(name = "QuartzCore", kind = "framework")]
    unsafe extern "C" {}

    // NSVisualEffectView constants.
    const BLEND_WITHIN_WINDOW: i64 = 1;
    const STATE_ACTIVE: i64 = 1;
    const MATERIAL_WINDOW_BACKGROUND: i64 = 12;
    // NSViewWidthSizable | NSViewHeightSizable
    const SIZABLE: u64 = 2 | 16;

    /// Gradient stops (position, mask alpha): fully applied at the window edge, easing to nothing.
    const STOPS: [(f64, f64); 5] = [(0.0, 1.0), (0.3, 0.85), (0.6, 0.45), (0.85, 0.12), (1.0, 0.0)];
    /// Opacity of the background-coloured tint inside each band.
    const TINT_ALPHA: f64 = 0.7;

    struct Band {
        view: id,
        tint: id,
        mask: id,
    }

    pub struct EdgeBlur {
        parent: id,
        top: Band,
        bottom: Band,
        top_h: f64,
        bottom_h: f64,
        /// Distance from the window's bottom edge to the bottom band (the toolbar height).
        bottom_inset: f64,
    }

    extern "C" fn hit_test_none(_: &Object, _: Sel, _: NSPoint) -> id {
        nil
    }

    /// An `NSVisualEffectView` that never receives mouse events, so clicks reach GPUI.
    fn passthrough_class() -> &'static Class {
        static REGISTER: Once = Once::new();
        REGISTER.call_once(|| {
            let mut decl = ClassDecl::new("AlphaEdgeBlurView", class!(NSVisualEffectView)).expect("class registered once");
            unsafe {
                decl.add_method(sel!(hitTest:), hit_test_none as extern "C" fn(&Object, Sel, NSPoint) -> id);
            }
            decl.register();
        });
        Class::get("AlphaEdgeBlurView").expect("registered above")
    }

    unsafe fn cg_color(r: f64, g: f64, b: f64, a: f64) -> id {
        let ns: id = msg_send![class!(NSColor), colorWithSRGBRed: r green: g blue: b alpha: a];
        msg_send![ns, CGColor]
    }

    unsafe fn make_band(parent: id, top_edge: bool) -> Band {
        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(10., 10.));
        unsafe {
            let view: id = msg_send![passthrough_class(), alloc];
            let view: id = msg_send![view, initWithFrame: frame];
            let _: () = msg_send![view, setBlendingMode: BLEND_WITHIN_WINDOW];
            let _: () = msg_send![view, setMaterial: MATERIAL_WINDOW_BACKGROUND];
            let _: () = msg_send![view, setState: STATE_ACTIVE];
            let _: () = msg_send![view, setWantsLayer: YES];

            let tint: id = msg_send![class!(NSView), alloc];
            let tint: id = msg_send![tint, initWithFrame: frame];
            let _: () = msg_send![tint, setWantsLayer: YES];
            let _: () = msg_send![tint, setAutoresizingMask: SIZABLE];
            let _: () = msg_send![view, addSubview: tint];

            // Alpha mask: opaque at the window edge, transparent toward the content.
            // The view's layer is not flipped, so y = 1 is the top.
            let mask: id = msg_send![class!(CAGradientLayer), layer];
            let colors: id = msg_send![class!(NSMutableArray), array];
            let locations: id = msg_send![class!(NSMutableArray), array];
            for (loc, alpha) in STOPS {
                let ns: id = msg_send![class!(NSColor), colorWithWhite: 0.0f64 alpha: alpha];
                let cg: id = msg_send![ns, CGColor];
                let _: () = msg_send![colors, addObject: cg];
                let n: id = msg_send![class!(NSNumber), numberWithDouble: loc];
                let _: () = msg_send![locations, addObject: n];
            }
            let _: () = msg_send![mask, setColors: colors];
            let _: () = msg_send![mask, setLocations: locations];
            let (start, end) = if top_edge {
                (NSPoint::new(0.5, 1.0), NSPoint::new(0.5, 0.0))
            } else {
                (NSPoint::new(0.5, 0.0), NSPoint::new(0.5, 1.0))
            };
            let _: () = msg_send![mask, setStartPoint: start];
            let _: () = msg_send![mask, setEndPoint: end];
            let layer: id = msg_send![view, layer];
            let _: () = msg_send![layer, setMask: mask];

            let _: () = msg_send![parent, addSubview: view];
            Band { view, tint, mask }
        }
    }

    impl EdgeBlur {
        pub fn install(window: &Window, top_h: f64, bottom_inset: f64, bottom_h: f64) -> Option<Self> {
            let handle = HasWindowHandle::window_handle(window).ok()?;
            let RawWindowHandle::AppKit(h) = handle.as_raw() else { return None };
            let parent = h.ns_view.as_ptr() as id;
            if parent.is_null() {
                return None;
            }
            unsafe {
                let top = make_band(parent, true);
                let bottom = make_band(parent, false);
                let this = Self { parent, top, bottom, top_h, bottom_h, bottom_inset };
                this.layout();
                Some(this)
            }
        }

        /// Repositions the bands for the current window size (cheap; call every frame).
        pub fn layout(&self) {
            unsafe {
                let bounds: NSRect = msg_send![self.parent, bounds];
                let flipped: BOOL = msg_send![self.parent, isFlipped];
                let flipped = flipped != NO;
                let (w, h) = (bounds.size.width, bounds.size.height);
                let top_y = if flipped { 0.0 } else { h - self.top_h };
                let bottom_y = if flipped { h - self.bottom_inset - self.bottom_h } else { self.bottom_inset };
                let _: () = msg_send![class!(CATransaction), begin];
                let _: () = msg_send![class!(CATransaction), setDisableActions: YES];
                for (band, y, height) in [(&self.top, top_y, self.top_h), (&self.bottom, bottom_y, self.bottom_h)] {
                    let frame = NSRect::new(NSPoint::new(0., y), NSSize::new(w, height));
                    let _: () = msg_send![band.view, setFrame: frame];
                    let local = NSRect::new(NSPoint::new(0., 0.), NSSize::new(w, height));
                    let _: () = msg_send![band.mask, setFrame: local];
                }
                let _: () = msg_send![class!(CATransaction), commit];
            }
        }

        /// Follows the app theme (which may differ from the system appearance).
        pub fn set_theme(&self, theme: &Theme) {
            let rgba = gpui::Rgba::from(theme.bg);
            unsafe {
                let name = NSString::alloc(nil).init_str(if theme.dark { "NSAppearanceNameDarkAqua" } else { "NSAppearanceNameAqua" });
                let appearance: id = msg_send![class!(NSAppearance), appearanceNamed: name];
                for band in [&self.top, &self.bottom] {
                    let _: () = msg_send![band.view, setAppearance: appearance];
                    let layer: id = msg_send![band.tint, layer];
                    let _: () = msg_send![layer, setBackgroundColor: cg_color(rgba.r as f64, rgba.g as f64, rgba.b as f64, TINT_ALPHA)];
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub use mac::EdgeBlur;

#[cfg(not(target_os = "macos"))]
pub struct EdgeBlur;

#[cfg(not(target_os = "macos"))]
impl EdgeBlur {
    pub fn install(_: &Window, _: f64, _: f64, _: f64) -> Option<Self> {
        None
    }
    pub fn layout(&self) {}
    pub fn set_theme(&self, _: &Theme) {}
}
