// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

/// How a field's name arrives: as text, or already classified for the section by a protocol's static table, as an HTTP/2 or HTTP/3 decoder can classify the fields it decodes by index.
///
/// Use [`StaticTableEntry::name`](crate::StaticTableEntry::name) to make the classified forms; a classification made for one section is meaningless for another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldName<'a> {
    /// A name to match against the section's declarations.
    Text(&'a str),
    /// A name the static table maps to one of the section's declarations, by id, with its text.
    Declaration(u16, &'a str),
    /// A name the static table finds the section forbids, with its text.
    Forbidden(&'a str),
    /// A name the static table finds the section neither declares nor forbids, with its text.
    Undeclared(&'a str),
}

impl<'a> FieldName<'a> {
    /// The name's text.
    pub fn as_str(&self) -> &'a str {
        match *self {
            Self::Text(name)
            | Self::Declaration(_, name)
            | Self::Forbidden(name)
            | Self::Undeclared(name) => name,
        }
    }
}
