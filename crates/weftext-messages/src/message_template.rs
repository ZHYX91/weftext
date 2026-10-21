/// This is the same placeholder grammar used by the canonical WebUI generator:
/// `{[A-Za-z][A-Za-z0-9]*}`. Other braces belong to literal author examples.
pub(crate) fn placeholders(template: &str) -> impl Iterator<Item = (std::ops::Range<usize>, &str)> {
    let mut offset = 0;
    std::iter::from_fn(move || {
        while let Some(relative) = template[offset..].find('{') {
            let start = offset + relative;
            offset = start + 1;
            let tail = &template[offset..];
            let Some(end) = tail.find('}') else { continue };
            let name = &tail[..end];
            let mut bytes = name.bytes();
            if bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
                && bytes.all(|byte| byte.is_ascii_alphanumeric())
            {
                offset += end + 1;
                return Some((start..offset, name));
            }
        }
        None
    })
}
