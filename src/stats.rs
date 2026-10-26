//! Word and character counting.

/// Number of whitespace-separated words.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Number of characters (Unicode scalar values), including whitespace.
pub fn char_count(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   \n\t "), 0);
        assert_eq!(word_count("hello"), 1);
        assert_eq!(word_count("hello  world\nfoo\tbar"), 4);
        assert_eq!(word_count("# Title\n\n- item one"), 5);
    }

    #[test]
    fn counts_chars() {
        assert_eq!(char_count(""), 0);
        assert_eq!(char_count("abc"), 3);
        assert_eq!(char_count("héllo"), 5);
        assert_eq!(char_count("a\nb"), 3);
    }
}
