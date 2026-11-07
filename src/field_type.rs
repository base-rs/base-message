// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::FieldValueError;

/// What an invalid value means for a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidValue {
    /// The section cannot be interpreted. This suits fields whose meaning matters for safety, such as `Content-Length`.
    Error,
    /// The value is skipped, as RFC 9110 permits for advisory fields such as `Accept`. A skipped singleton still counts as present, so a second instance is still a duplicate.
    Ignore,
}

/// One kind of field: how its instances combine, how one value parses, and what an invalid value means.
///
/// ```
/// use base_message::{FieldType, FieldValueError, ascii_text, combination::Singleton};
///
/// struct ContentLength(u64);
///
/// impl FieldType for ContentLength {
///     type Combination = Singleton;
///
///     fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
///         let text = ascii_text(value)?;
///         if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
///             return Err(FieldValueError::new("not a decimal length"));
///         }
///         text.parse().map(Self).map_err(|_| FieldValueError::new("length too large"))
///     }
/// }
/// ```
pub trait FieldType: Sized {
    /// One of the markers in [`combination`](crate::combination): [`Singleton`](crate::combination::Singleton), [`List`](crate::combination::List), or [`Separate`](crate::combination::Separate).
    type Combination;

    /// What an invalid value means, unless a declaration overrides it with `invalid = "..."`.
    const INVALID: InvalidValue = InvalidValue::Error;

    /// Parse one instance, or one element of a list, with surrounding spaces and tabs already removed.
    fn parse(value: &[u8]) -> Result<Self, FieldValueError>;
}
