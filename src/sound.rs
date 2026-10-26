//! Interface sounds: cues from cuelume (https://cuelume.dev, MIT), rendered
//! offline to WAV by `scripts/render-sounds.mjs` with the library's own
//! synthesis, embedded in the binary and played with `NSSound`.
//!
//! Each cue answers one moment: a line snapping into a block, an inline span
//! closing, a checkbox flipping, a file landing on disk. Nothing plays per
//! keystroke, on paste or on undo.

use crate::settings::Settings;
use gpui::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    /// A line became a heading (cuelume `select`, strong).
    Heading,
    /// A line became a list item, task, quote or divider (`select`).
    Block,
    /// Enter continued a list, task or quote (`select`, subtle).
    Item,
    /// Bold, italic, code, strike or a link closed (`tap`, subtle, keyboard).
    Inline,
    /// A fenced code block opened (`open`).
    Code,
    /// A task was checked (`toggle`).
    Check,
    /// A task was unchecked (`toggle`, going back).
    Uncheck,
    /// The document was saved (`success`, subtle).
    Saved,
    /// An export was written (`ready`).
    Exported,
}

macro_rules! clips {
    ($($name:ident = $file:literal),* $(,)?) => {
        $(const $name: &[u8] = include_bytes!(concat!("../assets/sounds/", $file, ".wav"));)*
    };
}

clips! {
    HEADING = "heading", BLOCK = "block", ITEM = "item",
    INLINE_1 = "inline-1", INLINE_2 = "inline-2", INLINE_3 = "inline-3", INLINE_4 = "inline-4",
    CODE = "code", CHECK = "check", UNCHECK = "uncheck", SAVED = "saved", EXPORTED = "exported",
}

impl Cue {
    /// The rendered takes of this cue; cues cuelume varies per play have several.
    fn takes(self) -> &'static [&'static [u8]] {
        match self {
            Cue::Heading => &[HEADING],
            Cue::Block => &[BLOCK],
            Cue::Item => &[ITEM],
            Cue::Inline => &[INLINE_1, INLINE_2, INLINE_3, INLINE_4],
            Cue::Code => &[CODE],
            Cue::Check => &[CHECK],
            Cue::Uncheck => &[UNCHECK],
            Cue::Saved => &[SAVED],
            Cue::Exported => &[EXPORTED],
        }
    }
}

/// Plays `cue` if sounds are switched on in settings.
pub fn play(cue: Cue, cx: &App) {
    if cx.global::<Settings>().sounds {
        platform::play(cue);
    }
}

#[cfg(target_os = "macos")]
// objc 0.2's macros test a `cargo-clippy` feature newer toolchains no longer know.
#[allow(unexpected_cfgs)]
mod platform {
    use super::Cue;
    use objc::runtime::{NO, Object};
    use objc::{class, msg_send, sel, sel_impl};
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Players per take: a cue that plays again while still ringing gets a
    /// second player rather than cutting the first off.
    const PLAYERS: usize = 3;

    struct Take {
        players: Vec<*mut Object>,
    }

    thread_local! {
        static TAKES: RefCell<HashMap<(Cue, usize), Take>> = RefCell::new(HashMap::new());
        static LAST: RefCell<HashMap<Cue, usize>> = RefCell::new(HashMap::new());
    }

    fn player(bytes: &'static [u8]) -> *mut Object {
        unsafe {
            // The clip is static, so NSData can borrow it.
            let data: *mut Object = msg_send![class!(NSData), dataWithBytesNoCopy: bytes.as_ptr() as *mut std::ffi::c_void length: bytes.len() freeWhenDone: NO];
            let sound: *mut Object = msg_send![class!(NSSound), alloc];
            msg_send![sound, initWithData: data]
        }
    }

    /// A different take from the last one, so repeats never sound identical.
    fn pick(cue: Cue, n: usize) -> usize {
        if n == 1 {
            return 0;
        }
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or(0);
        LAST.with(|last| {
            let mut last = last.borrow_mut();
            let prev = last.get(&cue).copied().unwrap_or(n);
            let mut i = seed % n;
            if i == prev {
                i = (i + 1) % n;
            }
            last.insert(cue, i);
            i
        })
    }

    pub fn play(cue: Cue) {
        let takes = cue.takes();
        let i = pick(cue, takes.len());
        TAKES.with(|map| {
            let mut map = map.borrow_mut();
            let take = map.entry((cue, i)).or_insert_with(|| Take { players: Vec::new() });
            unsafe {
                let idle = take.players.iter().copied().find(|p| {
                    let playing: bool = msg_send![*p, isPlaying];
                    !playing
                });
                let p = match idle {
                    Some(p) => p,
                    None if take.players.len() < PLAYERS => {
                        let p = player(takes[i]);
                        if p.is_null() {
                            return;
                        }
                        take.players.push(p);
                        p
                    }
                    None => {
                        let p = take.players[0];
                        take.players.rotate_left(1);
                        let _: bool = msg_send![p, stop];
                        p
                    }
                };
                let _: bool = msg_send![p, play];
            }
        });
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub fn play(_: super::Cue) {}
}
