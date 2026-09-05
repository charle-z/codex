/// Describes source-specific terms that should be rewritten in migrated artifacts.
#[derive(Clone, Copy)]
pub struct RewriteProfile {
    doc_file_name: &'static str,
    term_variants: &'static [&'static str],
    case_sensitive_term_variants: &'static [&'static str],
}

impl RewriteProfile {
    pub const fn new(doc_file_name: &'static str, term_variants: &'static [&'static str]) -> Self {
        Self {
            doc_file_name,
            term_variants,
            case_sensitive_term_variants: &[],
        }
    }

    pub const fn with_case_sensitive_term_variants(
        mut self,
        term_variants: &'static [&'static str],
    ) -> Self {
        self.case_sensitive_term_variants = term_variants;
        self
    }

    pub const fn doc_file_name(self) -> &'static str {
        self.doc_file_name
    }

    pub const fn term_variants(self) -> &'static [&'static str] {
        self.term_variants
    }

    pub const fn case_sensitive_term_variants(self) -> &'static [&'static str] {
        self.case_sensitive_term_variants
    }

    /// Rewrites source-specific documentation names and product terms to their Codex forms.
    pub fn rewrite(self, content: &str) -> String {
        let mut rewritten =
            replace_case_insensitive_with_boundaries(content, self.doc_file_name, "AGENTS.md");
        for from in self.term_variants {
            rewritten = replace_case_insensitive_with_boundaries(&rewritten, from, "Codex");
        }
        for from in self.case_sensitive_term_variants {
            rewritten = replace_with_boundaries(&rewritten, from, "Codex");
        }
        rewritten
    }
}

fn replace_with_boundaries(input: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return input.to_string();
    }

    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut last_emitted = 0usize;
    let mut search_start = 0usize;

    while let Some(relative_pos) = input[search_start..].find(needle) {
        let start = search_start + relative_pos;
        let end = start + needle.len();
        let boundary_before = start == 0 || !is_word_byte(bytes[start - 1]);
        let boundary_after = end == bytes.len() || !is_word_byte(bytes[end]);

        if boundary_before && boundary_after && !is_literal_reference_match(bytes, start, end) {
            output.push_str(&input[last_emitted..start]);
            output.push_str(replacement);
            last_emitted = end;
        }

        search_start = end;
    }

    if last_emitted == 0 {
        return input.to_string();
    }

    output.push_str(&input[last_emitted..]);
    output
}

fn replace_case_insensitive_with_boundaries(
    input: &str,
    needle: &str,
    replacement: &str,
) -> String {
    let needle_lower = needle.to_ascii_lowercase();
    if needle_lower.is_empty() {
        return input.to_string();
    }

    let haystack_lower = input.to_ascii_lowercase();
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut last_emitted = 0usize;
    let mut search_start = 0usize;

    while let Some(relative_pos) = haystack_lower[search_start..].find(&needle_lower) {
        let start = search_start + relative_pos;
        let end = start + needle_lower.len();
        let boundary_before = start == 0 || !is_word_byte(bytes[start - 1]);
        let boundary_after = end == bytes.len() || !is_word_byte(bytes[end]);

        if boundary_before && boundary_after && !is_literal_reference_match(bytes, start, end) {
            output.push_str(&input[last_emitted..start]);
            output.push_str(replacement);
            last_emitted = end;
        }

        search_start = start + 1;
    }

    if last_emitted == 0 {
        return input.to_string();
    }

    output.push_str(&input[last_emitted..]);
    output
}

/// Product names embedded in filesystem paths, URI references, Markdown link destinations, or
/// dotted identifiers are literal references to the source tool, not prose to retarget.
fn is_literal_reference_match(bytes: &[u8], start: usize, end: usize) -> bool {
    if let Some(before) = start.checked_sub(1).and_then(|idx| bytes.get(idx))
        && matches!(*before, b'/' | b'\\' | b'.')
    {
        return true;
    }

    if is_markdown_link_destination_match(bytes, start) {
        return true;
    }
    if is_uri_reference_match(bytes, start, end) {
        return true;
    }

    let Some(after) = bytes.get(end) else {
        return false;
    };
    if matches!(*after, b'/' | b'\\') {
        return true;
    }

    *after == b'.'
        && bytes
            .get(end + 1)
            .is_some_and(|byte| is_reference_suffix_byte(*byte))
}

fn is_uri_reference_match(bytes: &[u8], start: usize, end: usize) -> bool {
    if is_relative_uri_component_match(bytes, start, end) {
        return true;
    }

    if bytes.get(end) == Some(&b':')
        && bytes
            .get(end + 1)
            .is_some_and(|byte| is_uri_payload_start_byte(*byte))
        && is_uri_scheme(&bytes[start..end])
    {
        return true;
    }

    let token_start = bytes[..start]
        .iter()
        .rposition(u8::is_ascii_whitespace)
        .map_or(0, |idx| idx + 1);

    bytes[token_start..start]
        .iter()
        .enumerate()
        .any(|(offset, byte)| {
            if *byte != b':' {
                return false;
            }

            let colon = token_start + offset;
            let mut scheme_start = colon;
            while scheme_start > token_start && is_uri_scheme_byte(bytes[scheme_start - 1]) {
                scheme_start -= 1;
            }
            is_uri_scheme(&bytes[scheme_start..colon])
        })
}

fn is_relative_uri_component_match(bytes: &[u8], start: usize, end: usize) -> bool {
    let token_start = bytes[..start]
        .iter()
        .rposition(u8::is_ascii_whitespace)
        .map_or(0, |idx| idx + 1);

    if bytes[token_start..start]
        .iter()
        .any(|byte| matches!(*byte, b'?' | b'#'))
    {
        return true;
    }

    bytes.get(end).is_some_and(|byte| {
        matches!(*byte, b'?' | b'#')
            && bytes
                .get(end + 1)
                .is_some_and(|next| is_uri_payload_start_byte(*next))
    })
}

fn is_markdown_link_destination_match(bytes: &[u8], start: usize) -> bool {
    let line_start = bytes[..start]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |idx| idx + 1);
    let prefix = &bytes[line_start..start];

    if let Some(opener) = prefix.windows(2).rposition(|window| window == b"](")
        && prefix[..opener].contains(&b'[')
        && is_inline_markdown_destination_prefix(&prefix[opener + 2..])
    {
        return true;
    }

    if let Some(opener) = prefix.windows(2).rposition(|window| window == b"]:")
        && prefix[..opener].contains(&b'[')
    {
        return is_reference_markdown_destination_prefix(&prefix[opener + 2..]);
    }

    false
}

fn is_inline_markdown_destination_prefix(destination_prefix: &[u8]) -> bool {
    let mut nested_parentheses = 0usize;
    for (index, byte) in destination_prefix.iter().copied().enumerate() {
        if is_markdown_escaped(destination_prefix, index) {
            continue;
        }

        match byte {
            b'(' => nested_parentheses += 1,
            b')' if nested_parentheses > 0 => nested_parentheses -= 1,
            b')' => return false,
            byte if byte.is_ascii_whitespace() && nested_parentheses == 0 => return false,
            _ => {}
        }
    }
    true
}

fn is_reference_markdown_destination_prefix(destination_prefix: &[u8]) -> bool {
    let mut destination_start = 0usize;
    while destination_prefix
        .get(destination_start)
        .is_some_and(u8::is_ascii_whitespace)
    {
        destination_start += 1;
    }

    let angle_delimited = destination_prefix.get(destination_start) == Some(&b'<');
    if angle_delimited {
        destination_start += 1;
    }

    for (index, byte) in destination_prefix
        .iter()
        .copied()
        .enumerate()
        .skip(destination_start)
    {
        if is_markdown_escaped(destination_prefix, index) {
            continue;
        }

        if (angle_delimited && byte == b'>') || (!angle_delimited && byte.is_ascii_whitespace()) {
            return false;
        }
    }

    true
}

fn is_markdown_escaped(bytes: &[u8], index: usize) -> bool {
    let mut cursor = index;
    let mut preceding_backslashes = 0usize;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        preceding_backslashes += 1;
        cursor -= 1;
    }

    !preceding_backslashes.is_multiple_of(2)
}

fn is_uri_payload_start_byte(byte: u8) -> bool {
    is_reference_suffix_byte(byte)
        || matches!(
            byte,
            b'/' | b'.' | b'~' | b'%' | b':' | b'@' | b'?' | b'#' | b'[' | b'+'
        )
}

fn is_uri_scheme(bytes: &[u8]) -> bool {
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    first.is_ascii_alphabetic() && rest.iter().copied().all(is_uri_scheme_byte)
}

fn is_uri_scheme_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
}

fn is_reference_suffix_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
#[path = "rewrite_tests.rs"]
mod tests;
