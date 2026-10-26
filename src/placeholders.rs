//! Placeholder lines shown in an empty document. Edit freely.

pub const PLACEHOLDERS: [&str; 25] = [
    "The blank page blinked first.",
    "Once upon a deadline…",
    "Your inner critic is on a coffee break.",
    "Typos welcome. We'll fix them later.",
    "Begin anywhere. Editing is future-you's job.",
    "Words go here. Preferably good ones.",
    "Chapter One: Staring at the cursor.",
    "It was a dark and stormy draft…",
    "Draft now, delete bravely later.",
    "Somewhere, a semicolon is nervous.",
    "Write drunk on ideas, edit sober.",
    "A first sentence is just a rough guess.",
    "No pressure. Only a very patient cursor.",
    "Say the thing. Then say it better.",
    "Today's word count: ambitious.",
    "Dear diary, the cursor is judging me.",
    "Make it good. Or at least make it long.",
    "The muse is running late. Start without her.",
    "Plot twist: you start typing.",
    "Ink is cheap. Pixels are cheaper.",
    "Your masterpiece, loading…",
    "Empty page, infinite possibilities.",
    "Sentences are just thoughts wearing shoes.",
    "First draft: permission to be terrible.",
    "Type something. The page won't bite.",
];

/// A pseudo-random placeholder (time-seeded; no RNG dependency needed).
pub fn random() -> &'static str {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    PLACEHOLDERS[n % PLACEHOLDERS.len()]
}
