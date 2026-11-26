//! Checks for user-chosen text: names, reasons and descriptions must not hide or reorder
//! characters, or two names that look the same could differ, and text could be made to spoof.

/// True for characters that render as nothing or change how text is ordered: bidi controls,
/// zero-width and joiner characters, soft hyphen, filler characters, other format (Cf)
/// characters, private use, line and paragraph separators, and noncharacters.
/// (Surrogates cannot occur in a Rust `char`.)
pub fn is_hidden(c: char) -> bool {
    matches!(
        u32::from(c),
        0x00AD
            | 0x034F
            | 0x0600..=0x0605
            | 0x061C
            | 0x06DD
            | 0x070F
            | 0x0890..=0x0891
            | 0x08E2
            | 0x115F
            | 0x1160
            | 0x17B4..=0x17B5
            | 0x180B..=0x180F
            | 0x200B..=0x200F
            | 0x2028..=0x202E
            | 0x2060..=0x206F
            | 0x3164
            | 0xD800..=0xDFFF
            | 0xE000..=0xF8FF
            | 0xFE00..=0xFE0F
            | 0xFDD0..=0xFDEF
            | 0xFEFF
            | 0xFFA0
            | 0xFFF0..=0xFFFB
            | 0xFFFE..=0xFFFF
            | 0x110BD
            | 0x110CD
            | 0x13430..=0x1343F
            | 0x1BCA0..=0x1BCA3
            | 0x1D173..=0x1D17A
            | 0xE0000..=0xE0FFF
            | 0xF0000..=0x10FFFF
    )
}

/// `s` without its hidden characters.
pub fn strip_hidden(s: &str) -> String {
    s.chars().filter(|&c| !is_hidden(c)).collect()
}

/// True if `s` has a control character (`\n` allowed when `allow_newline`) or a hidden one.
pub fn has_unsafe(s: &str, allow_newline: bool) -> bool {
    s.chars()
        .any(|c| (c.is_control() && !(allow_newline && c == '\n')) || is_hidden(c))
}

/// A tag as stored and searched: trimmed, NFC-normalised (so `é` typed as one character or as
/// `e` and a combining accent is the same tag), lower case. The web's `cleanTag` does the same.
pub fn normalize_tag(raw: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    raw.trim()
        .nfc()
        .collect::<String>()
        .to_lowercase()
        .nfc()
        .collect()
}

/// What a tag is made of: letters, digits, combining marks (`\p{L}`, `\p{N}`, `\p{M}`), `-` and
/// `_`. The web's tag rule is the same.
pub fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || unicode_normalization::char::is_combining_mark(c) || c == '-' || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_bidi_and_invisible() {
        for c in [
            '\u{202A}',
            '\u{202E}',
            '\u{2066}',
            '\u{2069}',
            '\u{200B}',
            '\u{200F}',
            '\u{2060}',
            '\u{2064}',
            '\u{FEFF}',
            '\u{00AD}',
            '\u{034F}',
            '\u{061C}',
            '\u{115F}',
            '\u{1160}',
            '\u{3164}',
            '\u{FFA0}',
            '\u{2028}',
            '\u{2029}',
            '\u{E000}',
            '\u{F0000}',
            '\u{E0041}',
            '\u{FE0F}',
        ] {
            assert!(is_hidden(c), "{:04X}", u32::from(c));
        }
    }

    #[test]
    fn lets_normal_text_through() {
        for s in ["KEKW", "héllo wörld", "日本語", "😀", "a-b_c.1"] {
            assert!(!has_unsafe(s, false), "{s}");
        }
    }

    #[test]
    fn controls_and_newline() {
        assert!(has_unsafe("a\0b", true));
        assert!(has_unsafe("a\tb", true));
        assert!(has_unsafe("a\nb", false));
        assert!(!has_unsafe("a\nb", true));
        assert!(has_unsafe("a\u{202E}b", true));
    }

    #[test]
    fn strips() {
        assert_eq!(strip_hidden("KE\u{200B}KW\u{202E}"), "KEKW");
        assert_eq!(strip_hidden("\u{3164}"), "");
    }

    #[test]
    fn tags_are_nfc_and_allow_marks() {
        // "é" composed, and "e" with a combining acute, are one tag.
        assert_eq!(normalize_tag(" Caf\u{e9} "), "caf\u{e9}");
        assert_eq!(normalize_tag("CAFE\u{301}"), "caf\u{e9}");
        // Marks with no composed form stay, and count as tag characters.
        let t = normalize_tag("q\u{301}");
        assert_eq!(t.chars().count(), 2);
        assert!(t.chars().all(is_tag_char), "{t:?}");
        assert!("नमस्ते".chars().all(is_tag_char));
        assert!(!"a b".chars().all(is_tag_char));
        assert!(!"a!".chars().all(is_tag_char));
    }
}
