use sayn_formatter::format_source;
use std::fs;

#[test]
fn test_golden_formatter_sample() {
    let input = fs::read_to_string("tests/golden/formatter/sample_input.sayn")
        .expect("Failed to read sample_input.sayn");
    let expected = fs::read_to_string("tests/golden/formatter/sample_expected.sayn")
        .expect("Failed to read sample_expected.sayn");

    let formatted = format_source(&input);
    assert_eq!(
        formatted.trim(),
        expected.trim(),
        "Formatted output did not match expected golden file"
    );

    // Test idempotence: format(format(x)) == format(x)
    let reformatted = format_source(&formatted);
    assert_eq!(formatted, reformatted, "Formatter must be idempotent");
}

#[test]
fn test_unicode_and_mixed_language_formatting() {
    let input = "Title:\n- 🚀 Fast performance\n- 日本語のテキストサポート\n- Ümlauts and émojis 🌟\n\n```python\n# 🐍 Python code\nprint('hello')\n```\n";
    let formatted = format_source(input);
    assert!(formatted.contains("🚀 Fast performance"));
    assert!(formatted.contains("日本語のテキストサポート"));
    assert!(formatted.contains("Ümlauts and émojis 🌟"));
    assert!(formatted.contains("# 🐍 Python code"));
}
