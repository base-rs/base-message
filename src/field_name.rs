// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::__private::{IncomingField, IncomingName};
use crate::field_ref::IntoField;
use crate::{FieldSection, StaticTableEntry};
use std::fmt;

/// How a decoder with a static table names one field for section `S`: by an entry of the table, classified for `S`, or as text when the name was sent literally.
///
/// ```
/// use base_message::{FieldName, FieldSection, FieldType, FieldValueError, StaticTableEntry, ascii_text, combination::Singleton, static_table_entries};
///
/// struct Text(String);
///
/// impl FieldType<'_> for Text {
///     type Combination = Singleton;
///     fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
///         Ok(Self(ascii_text(value)?.to_owned()))
///     }
/// }
///
/// #[derive(FieldSection)]
/// struct Request {
///     #[field("host")]
///     host: Option<Text>,
///     #[field("x-request-id")]
///     request_id: Option<Text>,
/// }
///
/// const TABLE: [StaticTableEntry<Request>; 1] = static_table_entries::<Request, 1>(&["host"]);
/// let fields = [(FieldName::Indexed(TABLE[0]), &b"example.com"[..]), (FieldName::Text("x-request-id"), b"abc")];
/// let request = Request::interpret(fields).unwrap();
/// assert_eq!(request.request_id.map(|id| id.0).as_deref(), Some("abc"));
/// ```
pub enum FieldName<'a, S> {
    /// A name sent literally, to match against the section's declarations.
    Text(&'a str),
    /// A name sent as an index into the static table, classified for the section.
    Indexed(StaticTableEntry<S>),
}

impl<'a, S> FieldName<'a, S> {
    /// The name's text.
    pub fn as_str(&self) -> &'a str {
        match self {
            Self::Text(name) => name,
            Self::Indexed(entry) => entry.name(),
        }
    }
}

impl<S> Clone for FieldName<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for FieldName<'_, S> {}

impl<S> PartialEq for FieldName<'_, S> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Indexed(left), Self::Indexed(right)) => left == right,
            _ => false,
        }
    }
}

impl<S> Eq for FieldName<'_, S> {}

impl<S> fmt::Debug for FieldName<'_, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(name) => formatter.debug_tuple("Text").field(name).finish(),
            Self::Indexed(entry) => formatter.debug_tuple("Indexed").field(entry).finish(),
        }
    }
}

impl<'a, S: FieldSection<'a>> IntoField<'a, S> for (FieldName<'a, S::Identity>, &'a [u8]) {
    fn into_field(self, index: usize) -> IncomingField<'a> {
        let name = match self.0 {
            FieldName::Text(name) => IncomingName::Text(name),
            FieldName::Indexed(entry) => entry.incoming_name(),
        };
        IncomingField {
            position: index,
            name,
            value: self.1,
        }
    }
}
