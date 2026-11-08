// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::__private::{IncomingField, IncomingName};
use crate::FieldRef;
use crate::field_ref::{IntoField, debug_field};
use std::fmt;

/// One field, owned, with its position in the original section.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OwnedField {
    position: usize,
    name: String,
    value: Vec<u8>,
}

impl OwnedField {
    /// The field's position in the original section.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The name as it was written.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The value, as it was passed to the section.
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// The field, borrowed, so that another section can interpret it.
    pub fn as_field_ref(&self) -> FieldRef<'_> {
        FieldRef::new(self.position, &self.name, &self.value)
    }
}

/// Shows the value's length rather than its bytes, since values such as `Authorization` and `Cookie` carry credentials that must not reach logs.
impl fmt::Debug for OwnedField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_field(
            formatter,
            "OwnedField",
            self.position,
            &self.name,
            &self.value,
        )
    }
}

impl<'a> From<FieldRef<'a>> for OwnedField {
    fn from(field: FieldRef<'a>) -> Self {
        Self {
            position: field.position(),
            name: field.name().to_owned(),
            value: field.value().to_vec(),
        }
    }
}

impl<'a, S> IntoField<'a, S> for &'a OwnedField {
    fn into_field(self, _index: usize) -> IncomingField<'a> {
        IncomingField {
            position: self.position,
            name: IncomingName::Text(&self.name),
            value: &self.value,
        }
    }
}

/// A remainder that owns its fields, for keeping the fields a section does not take beyond the lifetime of the input, in order.
///
/// Use a `Vec<FieldRef>` instead to borrow them without copying. A section interprets owned fields by reference, keeping their original positions:
///
/// ```
/// use base_message::{FieldSection, FieldType, FieldValueError, OwnedFields, ascii_text, combination::Singleton};
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
/// struct Transport {
///     #[field("host")]
///     host: Text,
/// }
///
/// #[derive(FieldSection)]
/// struct Application {
///     #[field("x-request-id")]
///     request_id: Option<Text>,
/// }
///
/// let (_, rest): (Transport, OwnedFields) = Transport::partition([("Host", "example.com"), ("X-Request-Id", "abc")])?;
/// let application = Application::interpret(&rest)?;
/// assert_eq!(application.request_id.map(|id| id.0).as_deref(), Some("abc"));
/// # Ok::<(), base_message::FieldError>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OwnedFields {
    fields: Vec<OwnedField>,
}

impl OwnedFields {
    /// The fields, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, OwnedField> {
        self.fields.iter()
    }

    /// The fields, in order, as a slice.
    pub fn as_slice(&self) -> &[OwnedField] {
        &self.fields
    }

    /// The number of fields.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether there are no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// The values of every field named `name`, compared case-insensitively, in order.
    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.fields
            .iter()
            .filter(move |field| field.name.eq_ignore_ascii_case(name))
            .map(OwnedField::value)
    }
}

impl<'a> Extend<FieldRef<'a>> for OwnedFields {
    fn extend<I: IntoIterator<Item = FieldRef<'a>>>(&mut self, fields: I) {
        self.fields.extend(fields.into_iter().map(OwnedField::from));
    }
}

impl<'a> FromIterator<FieldRef<'a>> for OwnedFields {
    fn from_iter<I: IntoIterator<Item = FieldRef<'a>>>(fields: I) -> Self {
        Self {
            fields: fields.into_iter().map(OwnedField::from).collect(),
        }
    }
}

impl<'a> IntoIterator for &'a OwnedFields {
    type Item = &'a OwnedField;
    type IntoIter = std::slice::Iter<'a, OwnedField>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for OwnedFields {
    type Item = OwnedField;
    type IntoIter = std::vec::IntoIter<OwnedField>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}
