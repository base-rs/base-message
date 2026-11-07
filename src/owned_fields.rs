// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::FieldRef;

/// One field, owned, with its position in the original section.
#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// A remainder that owns its fields, for keeping the fields a section does not take beyond the lifetime of the input, in order.
///
/// Use a `Vec<FieldRef>` instead to borrow them without copying.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnedFields {
    fields: Vec<OwnedField>,
}

impl OwnedFields {
    /// The fields, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, OwnedField> {
        self.fields.iter()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

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

    /// The fields as borrowed [`FieldRef`]s with their original positions, so another section can interpret them.
    pub fn as_field_refs(&self) -> impl Iterator<Item = FieldRef<'_>> {
        self.fields.iter().map(|field| FieldRef {
            position: field.position,
            name: crate::FieldName::Text(&field.name),
            value: &field.value,
        })
    }
}

impl<'a> Extend<FieldRef<'a>> for OwnedFields {
    fn extend<I: IntoIterator<Item = FieldRef<'a>>>(&mut self, fields: I) {
        self.fields
            .extend(fields.into_iter().map(|field| OwnedField {
                position: field.position,
                name: field.name.as_str().to_owned(),
                value: field.value.to_vec(),
            }));
    }
}

impl<'a> IntoIterator for &'a OwnedFields {
    type Item = &'a OwnedField;
    type IntoIter = std::slice::Iter<'a, OwnedField>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
