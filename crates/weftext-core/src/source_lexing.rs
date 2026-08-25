use std::ops::Range;

pub(crate) fn split_comma_parts(
    source: &str,
    range: Range<usize>,
) -> Result<Vec<Range<usize>>, Range<usize>> {
    let mut parts = Vec::new();
    let mut part_start = range.start;
    let mut quoted = false;
    let mut escaped = false;
    for (relative, character) in source[range.clone()].char_indices() {
        let index = range.start + relative;
        if quoted && escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ',' if !quoted => {
                parts.push(part_start..index);
                part_start = index + 1;
            }
            _ => {}
        }
    }
    if quoted || escaped {
        return Err(part_start..range.end);
    }
    parts.push(part_start..range.end);
    Ok(parts)
}

pub(crate) fn find_unquoted_equals(source: &str, range: Range<usize>) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (relative, character) in source[range.clone()].char_indices() {
        let index = range.start + relative;
        if quoted && escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '=' if !quoted => return Some(index),
            _ => {}
        }
    }
    None
}

pub(crate) fn decode_attribute_value(source: &str, range: Range<usize>) -> Option<String> {
    if range.is_empty() {
        return None;
    }
    let raw = &source[range];
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw).ok();
    }
    if raw.contains(['"', '\'', '[', ']', '\\', '{', '}', '\n', '\r', '\t'])
        || raw.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some(raw.to_owned())
}

pub(crate) fn find_closing_bracket(source: &str, open: usize, limit: usize) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (relative, character) in source[open + 1..limit].char_indices() {
        let index = open + 1 + relative;
        if character == '\n' || character == '\r' {
            return None;
        }
        if quoted && escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ']' if !quoted => return Some(index),
            _ => {}
        }
    }
    None
}

pub(crate) fn complement_ranges(length: usize, protected: &[Range<u64>]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for protected in protected {
        let protected_start = usize::try_from(protected.start)
            .unwrap_or(length)
            .min(length);
        let protected_end = usize::try_from(protected.end).unwrap_or(length).min(length);
        if start < protected_start {
            ranges.push(start..protected_start);
        }
        start = start.max(protected_end);
    }
    if start < length {
        ranges.push(start..length);
    }
    ranges
}

pub(crate) fn line_end(source: &str, start: usize, limit: usize) -> usize {
    source[start..limit]
        .find(['\r', '\n'])
        .map_or(limit, |relative| start + relative)
}

pub(crate) fn trim_range(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && source.as_bytes()[range.start].is_ascii_whitespace() {
        range.start += 1;
    }
    while range.start < range.end && source.as_bytes()[range.end - 1].is_ascii_whitespace() {
        range.end -= 1;
    }
    range
}
