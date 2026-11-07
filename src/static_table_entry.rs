// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::{FieldName, FieldNames};

/// How one entry of a protocol's static table of field names relates to a section, as [`static_table_entries`] computes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticTableEntry {
    /// The section declares the name, with this declaration id.
    Declaration(u16),
    /// The section forbids the name.
    Forbidden,
    /// The section neither declares nor forbids the name.
    Undeclared,
}

impl StaticTableEntry {
    /// The [`FieldName`] a decoder passes for a field this entry names, with the entry's text.
    pub fn name(self, text: &str) -> FieldName<'_> {
        match self {
            Self::Declaration(declaration) => FieldName::Declaration(declaration, text),
            Self::Forbidden => FieldName::Forbidden(text),
            Self::Undeclared => FieldName::Undeclared(text),
        }
    }
}

/// Classify each entry of a protocol's static table of lowercase field names, such as HPACK's or QPACK's, for a section, at compile time, so a decoder can interpret indexed fields without comparing names.
///
/// ```
/// use base_message::{FieldSection, FieldType, FieldValueError, StaticTableEntry, ascii_text, combination::Singleton, static_table_entries};
///
/// struct Host(String);
///
/// impl FieldType for Host {
///     type Combination = Singleton;
///     fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
///         Ok(Self(ascii_text(value)?.to_owned()))
///     }
/// }
///
/// #[derive(FieldSection)]
/// #[fields(forbid("transfer-encoding"))]
/// struct Request {
///     #[field("host")]
///     host: Option<Host>,
/// }
///
/// const NAMES: [&str; 3] = ["host", "accept", "transfer-encoding"];
/// const TABLE: [StaticTableEntry; 3] = static_table_entries(&Request::NAMES, &NAMES);
/// assert_eq!(TABLE, [StaticTableEntry::Declaration(0), StaticTableEntry::Undeclared, StaticTableEntry::Forbidden]);
/// ```
///
/// # Panics
///
/// When the section has more than 65,536 declarations, which the derive's compile-time evaluation reports as an error.
pub const fn static_table_entries<const N: usize>(
    names: &FieldNames,
    table: &[&str; N],
) -> [StaticTableEntry; N] {
    let mut entries = [StaticTableEntry::Undeclared; N];
    let mut index = 0;
    while index < N {
        if let Some(declaration) = names.declaration(table[index]) {
            assert!(
                declaration <= u16::MAX as usize,
                "a section with a static table has at most 65,536 declarations"
            );
            entries[index] = StaticTableEntry::Declaration(declaration as u16);
        } else if names.forbids(table[index]) {
            entries[index] = StaticTableEntry::Forbidden;
        }
        index += 1;
    }
    entries
}
