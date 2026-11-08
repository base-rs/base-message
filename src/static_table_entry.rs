// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::__private::{IncomingField, IncomingName};
use crate::FieldSection;
use crate::field_ref::IntoField;
use std::fmt;
use std::marker::PhantomData;

/// How a name in a protocol's static table relates to a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Classification {
    /// The section declares the name, with this declaration id.
    Declaration(u16),
    /// The section forbids the name.
    Forbidden,
    /// The section neither declares nor forbids the name.
    Undeclared,
}

/// One entry of a protocol's static table of field names, classified for section `S` by [`static_table_entries`].
///
/// An entry can only be made by [`static_table_entries`], and a section accepts only entries made for it, so a decoder cannot pass one section's classification to another. Its name is the table's own, so it cannot disagree with the classification either.
pub struct StaticTableEntry<S> {
    classification: Classification,
    name: &'static str,
    section: PhantomData<fn() -> S>,
}

impl<S> StaticTableEntry<S> {
    /// How the section relates to this entry's name.
    pub const fn classification(&self) -> Classification {
        self.classification
    }

    /// The name in the table.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn incoming_name(self) -> IncomingName<'static> {
        match self.classification {
            Classification::Declaration(declaration) => IncomingName::Declaration(declaration),
            Classification::Forbidden => IncomingName::Forbidden(self.name),
            Classification::Undeclared => IncomingName::Undeclared(self.name),
        }
    }
}

impl<S> Clone for StaticTableEntry<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for StaticTableEntry<S> {}

impl<S> PartialEq for StaticTableEntry<S> {
    fn eq(&self, other: &Self) -> bool {
        (self.classification, self.name) == (other.classification, other.name)
    }
}

impl<S> Eq for StaticTableEntry<S> {}

impl<S> fmt::Debug for StaticTableEntry<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticTableEntry")
            .field("classification", &self.classification)
            .field("name", &self.name)
            .finish()
    }
}

impl<'a, S: FieldSection<'a>> IntoField<'a, S> for (StaticTableEntry<S::Identity>, &'a [u8]) {
    fn into_field(self, index: usize) -> IncomingField<'a> {
        IncomingField {
            position: index,
            name: self.0.incoming_name(),
            value: self.1,
        }
    }
}

/// Classify each name in a protocol's static table of lowercase field names, such as HPACK's or QPACK's, for section `S`, at compile time, so a decoder can interpret indexed fields without comparing names.
///
/// For a section with a lifetime, name it with `'static`, such as `Request<'static>`; its entries serve the section with any lifetime.
///
/// ```
/// use base_message::{Classification, FieldSection, FieldType, FieldValueError, StaticTableEntry, ascii_text, combination::Singleton, static_table_entries};
///
/// struct Host(String);
///
/// impl FieldType<'_> for Host {
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
/// const TABLE: [StaticTableEntry<Request>; 3] = static_table_entries::<Request, 3>(&NAMES);
/// let classifications = TABLE.map(|entry| entry.classification());
/// assert_eq!(classifications, [Classification::Declaration(0), Classification::Undeclared, Classification::Forbidden]);
///
/// let request = Request::interpret([(TABLE[0], &b"example.com"[..])]).unwrap();
/// assert_eq!(request.host.map(|host| host.0).as_deref(), Some("example.com"));
/// ```
///
/// # Panics
///
/// When a name in the table is not lowercase visible ASCII, such as `"Transfer-Encoding"`, since a section's names are lowercase and such a name would never match the declaration or forbidden name it spells; or when a name has a declaration id above 65,535. A constant reports either as a compile-time error.
pub const fn static_table_entries<S: FieldSection<'static>, const N: usize>(
    table: &[&'static str; N],
) -> [StaticTableEntry<S::Identity>; N] {
    let mut entries = [StaticTableEntry {
        classification: Classification::Undeclared,
        name: "",
        section: PhantomData,
    }; N];
    let mut index = 0;
    while index < N {
        assert!(
            lowercase(table[index]),
            "a static table's names must be lowercase visible ASCII, as HPACK's and QPACK's are"
        );
        let classification = if let Some(declaration) = S::NAMES.declaration(table[index]) {
            assert!(
                declaration <= u16::MAX as usize,
                "a static table classifies declaration ids up to 65,535"
            );
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the assertion above bounds the id"
            )]
            Classification::Declaration(declaration as u16)
        } else if S::NAMES.forbids(table[index]) {
            Classification::Forbidden
        } else {
            Classification::Undeclared
        };
        entries[index] = StaticTableEntry {
            classification,
            name: table[index],
            section: PhantomData,
        };
        index += 1;
    }
    entries
}

/// Whether a name is non-empty visible ASCII without uppercase letters. This admits field names and pseudo-header names such as `:authority`.
const fn lowercase(name: &str) -> bool {
    let bytes = name.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_graphic() || bytes[index].is_ascii_uppercase() {
            return false;
        }
        index += 1;
    }
    !bytes.is_empty()
}
