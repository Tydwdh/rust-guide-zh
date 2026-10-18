use ch11_demo::count_words;

#[test]
fn counts_words_in_a_sentence() {
    assert_eq!(count_words("the quick brown fox"), 4);
}
