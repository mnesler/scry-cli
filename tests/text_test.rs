use scry_cli::ui::text::wrap_text;

#[test]
fn test_wrap_text_basic() {
    let text = "Hello world this is a test";
    let wrapped = wrap_text(text, 15);

    assert_eq!(wrapped.len(), 2);
    assert_eq!(wrapped[0], "Hello world");
    assert_eq!(wrapped[1], "this is a test");
}

#[test]
fn test_wrap_text_single_line() {
    let text = "Short text";
    let wrapped = wrap_text(text, 20);

    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "Short text");
}

#[test]
fn test_wrap_text_empty_string() {
    let text = "";
    let wrapped = wrap_text(text, 20);

    // Empty text should return a single empty line
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "");
}

#[test]
fn test_wrap_text_width_zero() {
    let text = "Hello world";
    let wrapped = wrap_text(text, 0);

    // Width 0 returns the original text as-is
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "Hello world");
}

#[test]
fn test_wrap_text_long_single_word() {
    let text = "supercalifragilisticexpialidocious";
    let wrapped = wrap_text(text, 10);

    // A word longer than width should still be on its own line
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "supercalifragilisticexpialidocious");
}

#[test]
fn test_wrap_text_multiple_long_words() {
    let text = "supercalifragilisticexpialidocious antidisestablishmentarianism";
    let wrapped = wrap_text(text, 10);

    // Each long word should be on its own line
    assert_eq!(wrapped.len(), 2);
    assert_eq!(wrapped[0], "supercalifragilisticexpialidocious");
    assert_eq!(wrapped[1], "antidisestablishmentarianism");
}

#[test]
fn test_wrap_text_exact_width() {
    let text = "Hello";
    let wrapped = wrap_text(text, 5);

    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "Hello");
}

#[test]
fn test_wrap_text_multiple_spaces_normalized() {
    let text = "Hello    world";
    let wrapped = wrap_text(text, 20);

    // split_whitespace normalizes multiple spaces to single
    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "Hello world");
}

#[test]
fn test_wrap_text_width_40() {
    let text = "The quick brown fox jumps over the lazy dog and runs away into the forest";
    let wrapped = wrap_text(text, 40);

    assert!(wrapped.len() >= 2);
    for line in wrapped.iter() {
        // Each line should not exceed width (except for words longer than width)
        assert!(line.len() <= 40 || line.split_whitespace().count() == 1);
    }
}

#[test]
fn test_wrap_text_width_80() {
    let text = "The quick brown fox jumps over the lazy dog and runs away into the forest near the river";
    let wrapped = wrap_text(text, 80);

    // At width 80, this should fit on 2 lines or maybe 1
    assert!(wrapped.len() >= 1 && wrapped.len() <= 2);
}

#[test]
fn test_wrap_text_newlines_in_input() {
    // split_whitespace treats newlines as whitespace
    let text = "Hello\nworld";
    let wrapped = wrap_text(text, 20);

    assert_eq!(wrapped.len(), 1);
    assert_eq!(wrapped[0], "Hello world");
}

#[test]
fn test_wrap_text_preserves_order() {
    let text = "one two three four five";
    let wrapped = wrap_text(text, 10);

    // Join all lines and verify words are in order
    let rejoined: String = wrapped.join(" ");
    assert_eq!(rejoined, "one two three four five");
}
