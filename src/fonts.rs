//! The bundled default serif. To change the default font, edit this file and
//! the files under `assets/fonts/` — nothing else refers to a specific family.

use std::borrow::Cow;

/// Family name as reported by the bundled font files.
pub const SERIF_FAMILY: &str = "EB Garamond";

/// Bundled font files (regular, italic, semibold, semibold italic); strong spans use SemiBold, nothing is bolder.
pub fn bundled() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/EBGaramond-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/EBGaramond-Italic.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/EBGaramond-SemiBold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/EBGaramond-SemiBoldItalic.ttf").as_slice()),
    ]
}
