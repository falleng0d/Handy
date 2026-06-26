//! Final text transforms applied to the transcript after all processing.

/// Trailing word-separator punctuation (NOT `?`/`!`, which carry meaning).
fn is_trailing_separator(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | ';' | ':' | '…' | '。' | '，' | '；' | '：' | '、'
    )
}

/// Strip trailing word-separator punctuation (and any whitespace surrounding
/// it) from the end of `text`. Sentence-ending `?`/`!` are preserved. Only the
/// end of the string is affected.
pub fn strip_trailing_separators(text: &str) -> String {
    text.trim_end_matches(|c: char| c.is_whitespace() || is_trailing_separator(c))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_period() {
        assert_eq!(strip_trailing_separators("Hello world."), "Hello world");
    }

    #[test]
    fn strips_trailing_ellipsis_dots() {
        assert_eq!(strip_trailing_separators("Hello world..."), "Hello world");
    }

    #[test]
    fn strips_separator_with_trailing_whitespace() {
        assert_eq!(strip_trailing_separators("Hello, world; "), "Hello, world");
    }

    #[test]
    fn keeps_question_mark() {
        assert_eq!(strip_trailing_separators("Hello world?"), "Hello world?");
    }

    #[test]
    fn keeps_exclamation_mark() {
        assert_eq!(strip_trailing_separators("Hello world!"), "Hello world!");
    }

    #[test]
    fn strips_period_after_question_mark() {
        assert_eq!(strip_trailing_separators("Hello world?."), "Hello world?");
    }

    #[test]
    fn keeps_text_ending_in_exclamation_after_dots() {
        assert_eq!(strip_trailing_separators("wait...!"), "wait...!");
    }

    #[test]
    fn strips_trailing_colon() {
        assert_eq!(strip_trailing_separators("done:"), "done");
    }

    #[test]
    fn strips_cjk_full_stop() {
        assert_eq!(strip_trailing_separators("你好。"), "你好");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(strip_trailing_separators("plain text"), "plain text");
    }

    #[test]
    fn handles_empty_string() {
        assert_eq!(strip_trailing_separators(""), "");
    }

    #[test]
    fn consumes_whitespace_and_separator() {
        assert_eq!(strip_trailing_separators("hello . "), "hello");
    }

    #[test]
    fn keeps_fullwidth_question_mark() {
        assert_eq!(strip_trailing_separators("really？"), "really？");
    }
}
