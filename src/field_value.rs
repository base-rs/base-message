// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Helpers for parsing field values, which every protocol otherwise implements again.

use crate::FieldValueError;
use std::iter::FusedIterator;

/// Split one instance of a list field into its elements, as RFC 9110 section 5.6.1 describes: a comma inside a quoted string does not separate elements, spaces and tabs around each element are removed, and empty elements are skipped.
///
/// Comments, the parenthesized text some fields such as `Via` allow (RFC 9110 section 5.6.5), are not recognized, so a comma inside one separates elements. A field type whose values can contain comments should use [`Separate`](crate::combination::Separate) and split its values itself.
///
/// ```
/// use base_message::split_list;
///
/// let elements: Result<Vec<&[u8]>, _> = split_list(b"text/html, , application/json;q=\"0,9\"").collect();
/// assert_eq!(elements?, [&b"text/html"[..], &b"application/json;q=\"0,9\""[..]]);
/// assert!(split_list(b"a, \"b").any(|element| element.is_err()));
/// # Ok::<(), base_message::FieldValueError>(())
/// ```
///
/// The elements are produced without allocating. A quoted string that does not end is reported as an error after the elements before it, and then the iterator ends.
pub fn split_list(value: &[u8]) -> ListElements<'_> {
    ListElements { rest: Some(value) }
}

/// The elements of one instance of a list field, as [`split_list`] produces them.
#[derive(Clone, Debug)]
pub struct ListElements<'a> {
    /// The value not yet split, or `None` once the elements are exhausted.
    rest: Option<&'a [u8]>,
}

impl<'a> Iterator for ListElements<'a> {
    type Item = Result<&'a [u8], FieldValueError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(rest) = self.rest {
            let (mut quoted, mut escaped, mut separator) = (false, false, None);
            for (index, &byte) in rest.iter().enumerate() {
                if escaped {
                    escaped = false;
                } else if quoted && byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    quoted = !quoted;
                } else if byte == b',' && !quoted {
                    separator = Some(index);
                    break;
                }
            }
            let element = match separator {
                Some(index) => {
                    self.rest = Some(&rest[index + 1..]);
                    &rest[..index]
                }
                None => {
                    self.rest = None;
                    if quoted {
                        return Some(Err(FieldValueError::new("unterminated quoted string")));
                    }
                    rest
                }
            };
            let element = trim_whitespace(element);
            if !element.is_empty() {
                return Some(Ok(element));
            }
        }
        None
    }
}

impl FusedIterator for ListElements<'_> {}

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
    let invalid = FieldValueError::new("value is not visible ASCII");
    let text = std::str::from_utf8(value).map_err(|_| invalid)?;
    if !text
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ' || byte == b'\t')
    {
        return Err(invalid);
    }
    Ok(text)
}
