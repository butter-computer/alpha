//! Tiny subsequence-based fuzzy matcher used by the command palette.

/// Returns a score if every character of `query` appears in `candidate`, in
/// order (case-insensitive). Higher is better. An empty query matches all.
pub fn score(query: &str, candidate: &str) -> Option<i32> {
    let q: Vec<char> = query.chars().flat_map(char::to_lowercase).filter(|c| !c.is_whitespace()).collect();
    if q.is_empty() {
        return Some(0);
    }
    let cand: Vec<char> = candidate.chars().collect();
    let mut qi = 0;
    let mut total = 0;
    let mut prev: Option<usize> = None;
    for (i, c) in cand.iter().enumerate() {
        if qi == q.len() {
            break;
        }
        if c.to_lowercase().next() == Some(q[qi]) {
            let mut s = 1;
            if prev == Some(i.wrapping_sub(1)) && i > 0 {
                s += 5; // consecutive run
            }
            if i == 0 || !cand[i - 1].is_alphanumeric() {
                s += 8; // word start
            }
            if prev.is_none() {
                s -= (i as i32).min(10); // late first match
            }
            total += s;
            prev = Some(i);
            qi += 1;
        }
    }
    (qi == q.len()).then_some(total - (cand.len() as i32 / 8))
}

/// Filters and ranks items by `query`, best first; stable for equal scores.
pub fn filter<'a, T>(query: &str, items: &'a [T], text: impl Fn(&T) -> &str) -> Vec<&'a T> {
    let mut scored: Vec<(i32, usize, &T)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| score(query, text(item)).map(|s| (s, i, item)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(score("", "New").is_some());
    }

    #[test]
    fn subsequence_required() {
        assert!(score("sv", "Save").is_some());
        assert!(score("vs", "Save").is_none());
        assert!(score("xyz", "Save").is_none());
    }

    #[test]
    fn case_insensitive() {
        assert!(score("SAVE", "save as").is_some());
    }

    #[test]
    fn ranks_prefix_and_word_starts_higher() {
        let items = ["Export Markdown", "Save As…", "Save", "Toggle Theme"];
        let r = filter("save", &items, |s| s);
        assert_eq!(*r[0], "Save");
        assert_eq!(*r[1], "Save As…");
        let r = filter("tt", &items, |s| s);
        assert_eq!(*r[0], "Toggle Theme");
    }

    #[test]
    fn filter_drops_non_matches() {
        let items = ["New", "Open…"];
        assert_eq!(filter("zzz", &items, |s| s).len(), 0);
    }
}
