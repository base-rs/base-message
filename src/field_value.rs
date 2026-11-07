// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Helpers for parsing field values, which every protocol otherwise implements again.

use crate::FieldValueError;

/// Split one instance of a list field into its elements, as RFC 9110 section 5.6.1 describes: a comma inside a quoted string does not separate elements, spaces and tabs around each element are removed, and empty elements are skipped.
///
/// ```
/// use base_message::split_list;
///
/// let elements = split_list(b"text/html, , application/json;q=\"0,9\"")?;
/// assert_eq!(elements, [&b"text/html"[..], &b"application/json;q=\"0,9\""[..]]);
/// # Ok::<(), base_message::FieldValueError>(())
/// ```
///
/// # Errors
///
/// A quoted string that does not end.
pub fn split_list(value: &[u8]) -> Result<Vec<&[u8]>, FieldValueError> {
    let mut elements = Vec::new();
    let (mut start, mut quoted, mut escaped) = (0, false, false);
    for (index, &byte) in value.iter().enumerate() {
        if escaped {
            escaped = false;
        } else if quoted && byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            quoted = !quoted;
        } else if byte == b',' && !quoted {
            elements.push(&value[start..index]);
            start = index + 1;
        }
    }
    if quoted {
        return Err(FieldValueError::new("unterminated quoted string"));
    }
    elements.push(&value[start..]);
    Ok(elements
        .into_iter()
        .map(trim_whitespace)
        .filter(|element| !element.is_empty())
        .collect())
}

/// Remove spaces and tabs from both ends, the optional whitespace of RFC 9110 section 5.6.3.
pub fn trim_whitespace(value: &[u8]) -> &[u8] {
    let start = value
        .iter()
        .position(|byte| !matches!(byte, b' ' | b'\t'))
        .unwrap_or(value.len());
    let end = value
        .iter()
        .rposition(|byte| !matches!(byte, b' ' | b'\t'))
        .map_or(start, |index| index + 1);
    &value[start..end]
}

/// The value as text, when it is visible ASCII, spaces, and tabs, as most field grammars require.
///
/// # Errors
///
/// A control character or a byte outside ASCII.
pub fn ascii_text(value: &[u8]) -> Result<&str, FieldValueError> {
    if !value
        .iter()
        .all(|&byte| byte.is_ascii_graphic() || byte == b' ' || byte == b'\t')
    {
        return Err(FieldValueError::new("value is not visible ASCII"));
    }
    Ok(std::str::from_utf8(value).expect("ASCII is UTF-8"))
}
