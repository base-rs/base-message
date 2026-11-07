// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::FieldName;

/// One field, borrowed, with its position in the original section.
///
/// [`FieldSection::partition`](crate::FieldSection::partition) returns the fields a section does not take as `FieldRef`s, so that the next section reports errors at their original positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldRef<'a> {
    /// The field's position in the original section.
    pub position: usize,
    pub name: FieldName<'a>,
    pub value: &'a [u8],
}

/// Something a section can interpret as one field.
///
/// - A `(name, value)` pair, whose position is its place in the input.
/// - A `(FieldName, value)` pair from a decoder, likewise.
/// - A [`FieldRef`], which keeps its own position, such as one from a remainder.
pub trait IntoFieldRef<'a> {
    /// The field, given its index in the input.
    fn into_field_ref(self, index: usize) -> FieldRef<'a>;
}

impl<'a> IntoFieldRef<'a> for (&'a str, &'a [u8]) {
    fn into_field_ref(self, index: usize) -> FieldRef<'a> {
        FieldRef {
            position: index,
            name: FieldName::Text(self.0),
            value: self.1,
        }
    }
}

impl<'a> IntoFieldRef<'a> for (FieldName<'a>, &'a [u8]) {
    fn into_field_ref(self, index: usize) -> FieldRef<'a> {
        FieldRef {
            position: index,
            name: self.0,
            value: self.1,
        }
    }
}

impl<'a> IntoFieldRef<'a> for FieldRef<'a> {
    fn into_field_ref(self, _index: usize) -> FieldRef<'a> {
        self
    }
}
