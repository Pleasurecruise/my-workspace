use std::borrow::Cow;

/// Expand the bare triple-backtick wrapper accepted by Knowledge for embed examples.
/// Standard fenced code is left untouched, including examples inside longer fences.
pub fn normalize_embed_examples(source: &str) -> Cow<'_, str> {
    let mut lines: Vec<_> = source.split('\n').map(Cow::Borrowed).collect();
    let mut fence = None;
    let mut changed = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_end_matches('\r');
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() > 3 {
            index += 1;
            continue;
        }
        let Some(marker @ (b'`' | b'~')) = trimmed.bytes().next() else {
            index += 1;
            continue;
        };
        let length = trimmed.bytes().take_while(|byte| *byte == marker).count();
        if length < 3 {
            index += 1;
            continue;
        }
        let info = &trimmed[length..];
        if let Some((open_marker, open_length)) = fence {
            if marker == open_marker && length >= open_length && info.trim().is_empty() {
                fence = None;
            }
            index += 1;
            continue;
        }
        if line.trim_end() == "```"
            && lines.get(index + 1).is_some_and(|next| {
                let next = next.trim_end();
                next.get(..9)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("```embed:"))
                    && next[9..]
                        .bytes()
                        .all(|byte| byte.is_ascii_alphabetic() || byte == b'-')
                    && next.len() > 9
            })
            && let Some(close) =
                (index + 2..lines.len()).find(|offset| lines[*offset].trim_end() == "```")
            && lines
                .get(close + 1)
                .is_some_and(|line| line.trim_end() == "```")
        {
            lines[index] = Cow::Borrowed("````markdown");
            lines[close + 1] = Cow::Borrowed("````");
            changed = true;
            index = close + 2;
            continue;
        }
        if marker != b'`' || !info.contains('`') {
            fence = Some((marker, length));
        }
        index += 1;
    }
    if changed {
        Cow::Owned(lines.join("\n"))
    } else {
        Cow::Borrowed(source)
    }
}
