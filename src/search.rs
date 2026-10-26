//! Find-in-document matching (case-insensitive, literal).

use std::ops::Range;

/// All non-overlapping matches of `query` in `text`, as byte ranges.
pub fn find_all(text: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    // Lowercasing can change byte lengths, so only fold ASCII to keep offsets valid.
    let hay = text.to_ascii_lowercase();
    let needle = query.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = hay[from..].find(&needle) {
        let start = from + pos;
        let end = start + needle.len();
        if text.is_char_boundary(start) && text.is_char_boundary(end) {
            out.push(start..end);
        }
        from = end;
    }
    out
}

/// Index of the first match starting at or after `offset`, wrapping around.
pub fn next_index(matches: &[Range<usize>], offset: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().position(|m| m.start >= offset).unwrap_or(0))
}

/// Index of the last match starting before `offset`, wrapping around.
pub fn prev_index(matches: &[Range<usize>], offset: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().rposition(|m| m.start < offset).unwrap_or(matches.len() - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_case_insensitive() {
        assert_eq!(find_all("Foo foo FOO", "foo"), vec![0..3, 4..7, 8..11]);
    }

    #[test]
    fn empty_query_no_matches() {
        assert!(find_all("abc", "").is_empty());
    }

    #[test]
    fn non_overlapping() {
        assert_eq!(find_all("aaaa", "aa"), vec![0..2, 2..4]);
    }

    #[test]
    fn unicode_offsets_valid() {
        let t = "héllo hé";
        let m = find_all(t, "hé");
        assert_eq!(m.len(), 2);
        assert_eq!(&t[m[1].clone()], "hé");
    }

    #[test]
    fn next_prev_wrap() {
        let m = vec![0..1, 5..6, 9..10];
        assert_eq!(next_index(&m, 0), Some(0));
        assert_eq!(next_index(&m, 1), Some(1));
        assert_eq!(next_index(&m, 10), Some(0));
        assert_eq!(prev_index(&m, 5), Some(0));
        assert_eq!(prev_index(&m, 0), Some(2));
        assert_eq!(next_index(&[], 0), None);
    }
}
