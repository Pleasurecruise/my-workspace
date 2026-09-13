use super::{EmbedError, escape_html};

pub(super) fn render(source: &str) -> Result<String, EmbedError> {
    let invalid = || {
        EmbedError::InvalidDocument(
            "expected a complete unified text patch with matching hunk counts",
        )
    };
    let mut old_remaining = 0_u64;
    let mut new_remaining = 0_u64;
    let mut file = false;
    let mut old_header = false;
    let mut hunk = false;
    let mut file_hunk = false;
    let mut previous_content = false;
    let mut result = String::new();
    for text in source.split('\n') {
        let mut kind = "header";
        if text == "\\ No newline at end of file" {
            if !previous_content {
                return Err(invalid());
            }
            previous_content = false;
        } else if old_remaining > 0 || new_remaining > 0 {
            let prefix = text.as_bytes().first().ok_or_else(invalid)?;
            if !matches!(prefix, b' ' | b'+' | b'-') {
                return Err(invalid());
            }
            if *prefix != b'+' {
                old_remaining = old_remaining.checked_sub(1).ok_or_else(invalid)?;
            }
            if *prefix != b'-' {
                new_remaining = new_remaining.checked_sub(1).ok_or_else(invalid)?;
            }
            kind = match prefix {
                b'+' => "add",
                b'-' => "remove",
                _ => "context",
            };
            previous_content = true;
        } else {
            previous_content = false;
            if let Some(header) = text.strip_prefix("@@ -") {
                if !file || old_header {
                    return Err(invalid());
                }
                let (ranges, suffix) = header.split_once(" @@").ok_or_else(invalid)?;
                if !suffix.is_empty() && !suffix.starts_with(' ') {
                    return Err(invalid());
                }
                let (old, new) = ranges.split_once(" +").ok_or_else(invalid)?;
                let count = |range: &str| -> Result<u64, EmbedError> {
                    let (start, count) = range.split_once(',').unwrap_or((range, "1"));
                    for number in [start, count] {
                        if number.is_empty() || !number.bytes().all(|c| c.is_ascii_digit()) {
                            return Err(invalid());
                        }
                        if number.parse::<u64>().map_err(|_| invalid())? > 9_007_199_254_740_991 {
                            return Err(invalid());
                        }
                    }
                    count.parse().map_err(|_| invalid())
                };
                old_remaining = count(old)?;
                new_remaining = count(new)?;
                if old_remaining + new_remaining == 0 {
                    return Err(invalid());
                }
                hunk = true;
                file_hunk = true;
            } else if text.starts_with("--- ") && text.len() > 4 {
                if old_header || (file && !file_hunk) {
                    return Err(invalid());
                }
                old_header = true;
                file = false;
                file_hunk = false;
            } else if text.starts_with("+++ ") && text.len() > 4 {
                if !old_header {
                    return Err(invalid());
                }
                old_header = false;
                file = true;
            } else if [
                "diff --git ",
                "index ",
                "new file mode ",
                "deleted file mode ",
                "old mode ",
                "new mode ",
                "similarity index ",
                "rename from ",
                "rename to ",
                "copy from ",
                "copy to ",
            ]
            .iter()
            .any(|prefix| {
                text.strip_prefix(prefix)
                    .is_some_and(|tail| !tail.is_empty())
            }) {
                if old_header || (file && !file_hunk) {
                    return Err(invalid());
                }
                if text.starts_with("diff --git ") {
                    file = false;
                    file_hunk = false;
                }
            } else {
                return Err(invalid());
            }
        }
        result.push_str(&format!(
            "<span class=\"diff-{kind}\">{}\n</span>",
            escape_html(text)
        ));
    }
    if !hunk || !file || !file_hunk || old_header || old_remaining != 0 || new_remaining != 0 {
        return Err(invalid());
    }
    Ok(result)
}
