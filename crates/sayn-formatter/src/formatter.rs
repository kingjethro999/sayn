pub fn format_source(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();

    let mut output = Vec::new();
    let mut in_code_block = false;
    let mut prev_was_empty = false;

    for (idx, line) in lines.iter().enumerate() {
        // If we are on the very last empty split from a trailing newline, handle properly
        if idx == lines.len() - 1 && line.is_empty() {
            break;
        }

        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            output.push(line.trim_end().to_string());
            prev_was_empty = false;
            continue;
        }

        if in_code_block {
            // Inside code block: preserve exact bytes/whitespace
            output.push((*line).to_string());
            prev_was_empty = false;
            continue;
        }

        let trimmed_end = line.trim_end();
        let trimmed = trimmed_end.trim();

        if trimmed.is_empty() {
            if !prev_was_empty && !output.is_empty() {
                output.push(String::new());
                prev_was_empty = true;
            }
            continue;
        }

        // Check if line is a section heading (ends with : or starts with #)
        let is_heading = trimmed.starts_with('#')
            || (trimmed.ends_with(':')
                && trimmed.split_whitespace().count() <= 4
                && !trimmed.starts_with('-')
                && !trimmed.starts_with('*'));

        if is_heading && !output.is_empty() && !prev_was_empty {
            // Ensure single blank line before heading
            output.push(String::new());
        }

        // Normalize list items: standardize `* ` to `- `
        let formatted_line = if let Some(stripped) = trimmed.strip_prefix("* ") {
            let indent = trimmed_end
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();
            format!("{}- {}", indent, stripped.trim())
        } else if let Some(stripped) = trimmed.strip_prefix("- ") {
            let indent = trimmed_end
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();
            format!("{}- {}", indent, stripped.trim())
        } else {
            trimmed_end.to_string()
        };

        output.push(formatted_line);
        prev_was_empty = false;
    }

    // Ensure single trailing newline
    let mut result = output.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

pub fn check_source(input: &str) -> bool {
    let formatted = format_source(input);
    input == formatted
}

pub fn diff_source(original: &str, formatted: &str) -> String {
    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    let mut diff = String::new();
    let max_len = orig_lines.len().max(fmt_lines.len());

    for i in 0..max_len {
        let orig = orig_lines.get(i).copied();
        let fmt = fmt_lines.get(i).copied();

        match (orig, fmt) {
            (Some(o), Some(f)) if o != f => {
                diff.push_str(&format!("- {}\n", o));
                diff.push_str(&format!("+ {}\n", f));
            }
            (Some(o), None) => {
                diff.push_str(&format!("- {}\n", o));
            }
            (None, Some(f)) => {
                diff.push_str(&format!("+ {}\n", f));
            }
            _ => {}
        }
    }

    diff
}
