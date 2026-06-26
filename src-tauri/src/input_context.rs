//! Trims focused-field text injected via the `${input}` post-processing
//! placeholder down to its tail, so a long document doesn't blow up the prompt.

/// Keep only the tail of `text` according to the enabled limits (0 = unlimited).
/// Limits are measured from the END of the text; whichever limit is hit first
/// (i.e. produces the shortest tail) wins. CRLF/CR are normalized to LF before
/// counting and in the returned text. When all limits are 0, the original text
/// is returned unchanged (no normalization).
pub fn limit_context(
    text: &str,
    max_paragraphs: usize,
    max_lines: usize,
    max_chars: usize,
) -> String {
    if max_paragraphs == 0 && max_lines == 0 && max_chars == 0 {
        return text.to_string();
    }

    // Normalize line endings for consistent counting and output.
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let chars: Vec<char> = normalized.chars().collect();

    // Each enabled limit yields a start index (char offset); the largest start
    // (shortest tail) wins.
    let mut start = 0usize;
    if max_chars > 0 {
        start = start.max(chars.len().saturating_sub(max_chars));
    }
    if max_lines > 0 {
        start = start.max(line_start(&chars, max_lines));
    }
    if max_paragraphs > 0 {
        start = start.max(paragraph_start(&chars, max_paragraphs));
    }

    chars[start..].iter().collect()
}

/// Char index of the start of the last `n` lines (separator = single `\n`).
fn line_start(chars: &[char], n: usize) -> usize {
    let mut count = 0usize;
    let mut idx = chars.len();
    while idx > 0 {
        idx -= 1;
        if chars[idx] == '\n' {
            count += 1;
            if count == n {
                return idx + 1;
            }
        }
    }
    0
}

/// Char index of the start of the last `n` paragraphs (separator = a run of
/// 2+ consecutive `\n`).
fn paragraph_start(chars: &[char], n: usize) -> usize {
    let mut boundaries = 0usize;
    let mut idx = chars.len();
    while idx > 0 {
        idx -= 1;
        if chars[idx] == '\n' {
            // Expand to the full run of consecutive newlines.
            let run_end = idx;
            let mut run_start = idx;
            while run_start > 0 && chars[run_start - 1] == '\n' {
                run_start -= 1;
            }
            if run_end - run_start + 1 >= 2 {
                boundaries += 1;
                if boundaries == n {
                    return run_end + 1;
                }
            }
            idx = run_start; // skip the whole run (loop will decrement past it)
        }
    }
    0
}

const OPEN_TAG: &str = "<on input>";
const CLOSE_TAG: &str = "</on input>";

fn strip_one_leading_newline(s: &str) -> &str {
    s.strip_prefix("\r\n")
        .or_else(|| s.strip_prefix('\n'))
        .unwrap_or(s)
}

fn strip_one_trailing_newline(s: &str) -> &str {
    s.strip_suffix("\r\n")
        .or_else(|| s.strip_suffix('\n'))
        .unwrap_or(s)
}

/// Render `<on input>...</on input>` sections and the `${input}` placeholder.
///
/// - The `<on input>` / `</on input>` tags are always removed.
/// - When `input_available` is false, each section's inner content is removed
///   entirely; otherwise the inner content is kept.
/// - Remaining `${input}` placeholders are replaced with `input_context`
///   (empty string when input is not available).
/// - A missing closing tag is treated as a section running to the end of the prompt.
pub fn render_input(prompt: &str, input_context: &str, input_available: bool) -> String {
    let mut out = String::with_capacity(prompt.len());
    let mut rest = prompt;

    while let Some(open_idx) = rest.find(OPEN_TAG) {
        out.push_str(&rest[..open_idx]);
        let after_open = &rest[open_idx + OPEN_TAG.len()..];
        let (inner, after_close) = match after_open.find(CLOSE_TAG) {
            Some(close_rel) => (
                &after_open[..close_rel],
                &after_open[close_rel + CLOSE_TAG.len()..],
            ),
            None => (after_open, ""), // missing close: section runs to end
        };

        if input_available {
            // Keep the inner content (tags removed); trim one surrounding
            // newline so the tags' own lines don't leave blank gaps.
            out.push_str(strip_one_trailing_newline(strip_one_leading_newline(inner)));
            rest = after_close;
        } else {
            // Drop the section; also drop one newline that followed the close
            // tag so the removed block doesn't leave a blank line.
            rest = strip_one_leading_newline(after_close);
        }
    }
    out.push_str(rest);

    let replacement = if input_available { input_context } else { "" };
    out.replace("${input}", replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_zero_returns_unchanged_preserving_crlf() {
        assert_eq!(limit_context("a\r\nb", 0, 0, 0), "a\r\nb");
        assert_eq!(limit_context("anything\r\rhere", 0, 0, 0), "anything\r\rhere");
    }

    #[test]
    fn last_n_chars() {
        assert_eq!(limit_context("hello world", 0, 0, 5), "world");
        // X larger than text => unchanged content.
        assert_eq!(limit_context("hi", 0, 0, 50), "hi");
    }

    #[test]
    fn last_n_chars_counts_unicode_scalars() {
        // Four-char string of multi-byte chars; keep last 2 chars.
        assert_eq!(limit_context("áéíó", 0, 0, 2), "íó");
    }

    #[test]
    fn last_n_lines() {
        assert_eq!(limit_context("l1\nl2\nl3\nl4", 0, 2, 0), "l3\nl4");
        // N >= line count => whole (normalized) text.
        assert_eq!(limit_context("l1\nl2", 0, 5, 0), "l1\nl2");
    }

    #[test]
    fn last_n_paragraphs() {
        assert_eq!(limit_context("p1\n\np2\n\np3", 1, 0, 0), "p3");
        assert_eq!(limit_context("p1\n\np2\n\np3", 2, 0, 0), "p2\n\np3");
    }

    #[test]
    fn crlf_lines() {
        assert_eq!(limit_context("l1\r\nl2\r\nl3", 0, 2, 0), "l2\nl3");
    }

    #[test]
    fn crlf_paragraphs() {
        assert_eq!(limit_context("p1\r\n\r\np2\r\n\r\np3", 1, 0, 0), "p3");
    }

    #[test]
    fn multiple_limits_most_restrictive_wins() {
        // chars (4) beats lines (2): "cccc" is shorter than "bbbb\ncccc".
        assert_eq!(limit_context("aaaa\nbbbb\ncccc", 0, 2, 4), "cccc");
    }

    #[test]
    fn runs_of_three_plus_newlines_are_one_boundary() {
        // Three newlines between p1 and p2 is a single paragraph boundary.
        assert_eq!(limit_context("p1\n\n\np2", 1, 0, 0), "p2");
        assert_eq!(limit_context("p1\n\n\np2", 2, 0, 0), "p1\n\n\np2");
    }

    #[test]
    fn render_own_line_section_available() {
        let prompt = "Clean this.\n<on input>\nContext:\n${input}\n</on input>\nTranscript:\n${output}";
        assert_eq!(
            render_input(prompt, "DOC", true),
            "Clean this.\nContext:\nDOC\nTranscript:\n${output}"
        );
    }

    #[test]
    fn render_own_line_section_not_available() {
        let prompt = "Clean this.\n<on input>\nContext:\n${input}\n</on input>\nTranscript:\n${output}";
        assert_eq!(
            render_input(prompt, "", false),
            "Clean this.\nTranscript:\n${output}"
        );
    }

    #[test]
    fn render_input_outside_section() {
        assert_eq!(render_input("ctx: ${input}", "D", true), "ctx: D");
        assert_eq!(render_input("ctx: ${input}", "", false), "ctx: ");
    }

    #[test]
    fn render_inline_tags_available() {
        assert_eq!(
            render_input("a <on input>x ${input}</on input> b", "D", true),
            "a x D b"
        );
    }

    #[test]
    fn render_tags_always_removed() {
        let prompt = "a <on input>x ${input}</on input> b";
        for &(ctx, available) in &[("D", true), ("", false)] {
            let out = render_input(prompt, ctx, available);
            assert!(!out.contains("<on input>"), "open tag leaked: {out:?}");
            assert!(!out.contains("</on input>"), "close tag leaked: {out:?}");
        }
    }

    #[test]
    fn render_multiple_sections() {
        let prompt = "start <on input>one ${input}</on input> mid <on input>two ${input}</on input> end";
        assert_eq!(
            render_input(prompt, "D", true),
            "start one D mid two D end"
        );
        assert_eq!(render_input(prompt, "", false), "start  mid  end");
    }

    #[test]
    fn render_missing_close_tag() {
        let prompt = "keep\n<on input>\nonly if input ${input}";
        let available = render_input(prompt, "D", true);
        assert!(available.contains("keep"));
        assert!(available.contains("only if input D"));
        assert!(!available.contains("<on input>"));

        let unavailable = render_input(prompt, "", false);
        assert_eq!(unavailable, "keep\n");
        assert!(unavailable.contains("keep"));
        assert!(!unavailable.contains("<on input>"));
        assert!(!unavailable.contains("${input}"));
    }

    #[test]
    fn render_crlf_section() {
        let prompt = "A\r\n<on input>\r\n${input}\r\n</on input>\r\nB";
        let out = render_input(prompt, "D", true);
        assert!(out.contains('D'));
        assert!(out.contains('A'));
        assert!(out.contains('B'));
        assert!(!out.contains("<on input>"));
        assert!(!out.contains("</on input>"));
        assert!(!out.contains("\r\n\r\n"));
    }

    #[test]
    fn render_no_tags_no_placeholder_unchanged() {
        assert_eq!(render_input("just a prompt", "D", true), "just a prompt");
        assert_eq!(render_input("just a prompt", "", false), "just a prompt");
    }
}
