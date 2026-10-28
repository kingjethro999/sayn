pub mod formatter;

pub use formatter::{check_source, diff_source, format_source};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_preserves_code_fences() {
        let input = "Title:\n- item 1  \n\n```js\n  const x = 1;   \n```\n";
        let formatted = format_source(input);
        assert!(formatted.contains("  const x = 1;   "));
    }

    #[test]
    fn test_format_trims_trailing_whitespace() {
        let input = "Hello world!    \nLine 2   \n";
        let formatted = format_source(input);
        assert_eq!(formatted, "Hello world!\nLine 2\n");
    }

    #[test]
    fn test_check_source_detects_unformatted() {
        let input = "Hello   \n";
        assert!(!check_source(input));
        let formatted = format_source(input);
        assert!(check_source(&formatted));
    }
}
