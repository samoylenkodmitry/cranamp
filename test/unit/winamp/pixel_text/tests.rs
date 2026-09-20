use super::*;
#[test]
fn native_text_preserves_the_title_case() {
    assert_ne!(glyph('a'), glyph('A'));
    for ch in 'a'..='z' {
        assert!(glyph(ch).is_some());
    }
}
#[test]
#[cfg(not(target_os = "ios"))]
fn the_small_face_is_four_wide_and_has_no_lower_case() {
    for ch in ('A'..='Z').chain('0'..='9') {
        let rows = small_glyph(ch).expect("every letter and digit");
        assert!(rows.iter().all(|bits| *bits < 16), "{ch} is wider than 4px");
        assert_eq!(small_glyph(ch.to_ascii_lowercase()), Some(rows));
    }
    assert_eq!(small_glyph('@'), None);
    for (a, b) in [('M', 'H'), ('W', 'H'), ('M', 'W')] {
        assert_ne!(
            small_glyph(a),
            small_glyph(b),
            "{a} and {b} have to be told apart at four pixels"
        );
    }
}
#[test]
fn native_text_clips_and_retains_unicode_fallback() {
    assert!(render("A VERY LONG TITLE", 7, 10, [1, 2, 3, 255]).is_some());
    assert!(render("音楽", 40, 10, [1, 2, 3, 255]).is_none());
    assert!(render("ABC", 0, 10, [1, 2, 3, 255]).is_none());
    assert!(render("Cranamp 01 - Retro Tracker", 150, 10, [1, 2, 3, 255]).is_some());
}
