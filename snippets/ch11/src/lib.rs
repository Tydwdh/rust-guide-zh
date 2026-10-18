/// 统计 `text` 中的单词个数（以空白分隔）。
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words() {
        assert_eq!(count_words("hello world"), 2);
    }

    #[test]
    fn ignores_extra_spaces() {
        assert_eq!(count_words("  hello   world  "), 2);
    }

    #[test]
    fn empty_text() {
        assert_eq!(count_words(""), 0);
    }
}
