// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::__private::{IncomingField, IncomingName};

/// One field, borrowed, with its position in the original section.
///
/// [`FieldSection::partition`](crate::FieldSection::partition) returns the fields a section does not take as `FieldRef`s, so that the next section reports errors at their original positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldRef<'a> {
    position: usize,
    name: &'a str,
    value: &'a [u8],
}

impl<'a> FieldRef<'a> {
    /// A field at a position in the original section.
    pub const fn new(position: usize, name: &'a str, value: &'a [u8]) -> Self {
        Self {
            position,
            name,
            value,
        }
    }

    /// The field's position in the original section.
    pub const fn position(&self) -> usize {
        self.position
    }

    /// The name as it was written.
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// The value, as it was passed to the section.
    pub const fn value(&self) -> &'a [u8] {
        self.value
    }
}

/// Something section `S` can interpret as one field.
///
/// - A `(name, value)` pair, with the value as bytes or text, whose position is its place in the input.
/// - A [`FieldRef`], which keeps its own position, such as one from a remainder, and an [`OwnedField`](crate::OwnedField) by reference likewise.
/// - A [`StaticTableEntry`](crate::StaticTableEntry) or [`FieldName`](crate::FieldName) with its value, classified by a static table built for `S`.
///
/// It is implemented only by this crate.
pub trait IntoField<'a, S> {
    #[doc(hidden)]
    fn into_field(self, index: usize) -> IncomingField<'a>;
}

impl<'a, S> IntoField<'a, S> for (&'a str, &'a [u8]) {
    fn into_field(self, index: usize) -> IncomingField<'a> {
        IncomingField {
            position: index,
            name: IncomingName::Text(self.0),
            value: self.1,
        }
    }
}

impl<'a, S> IntoField<'a, S> for (&'a str, &'a str) {
    fn into_field(self, index: usize) -> IncomingField<'a> {
        IncomingField {
            position: index,
            name: IncomingName::Text(self.0),
            value: self.1.as_bytes(),
        }
    }
}

impl<'a, S> IntoField<'a, S> for &(&'a str, &'a [u8]) {
    fn into_field(self, index: usize) -> IncomingField<'a> {
        IncomingField {
            position: index,
            name: IncomingName::Text(self.0),
            value: self.1,
        }
    }
}

impl<'a, S> IntoField<'a, S> for FieldRef<'a> {
    fn into_field(self, _index: usize) -> IncomingField<'a> {
        IncomingField {
            position: self.position,
            name: IncomingName::Text(self.name),
            value: self.value,
        }
    }
}

impl<'a, S> IntoField<'a, S> for &FieldRef<'a> {
    fn into_field(self, index: usize) -> IncomingField<'a> {
        <FieldRef<'a> as IntoField<'a, S>>::into_field(*self, index)
    }
}
