// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::FieldValueError;
use std::fmt;

/// Why a field section could not be interpreted, naming the field and its position in the original section.
///
/// The name is the section's own lowercase spelling of a declared or forbidden field. The name of an unknown field is untrusted input, so it is never included; nor is any value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldError {
    name: Option<&'static str>,
    position: Option<usize>,
    kind: FieldErrorKind,
}

/// The kind of [`FieldError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldErrorKind {
    /// A singleton field appeared again.
    Duplicate {
        /// The position of the field's first instance.
        first: usize,
    },
    /// A required field is absent.
    Missing,
    /// A value is invalid, and the field treats an invalid value as an error.
    Invalid(FieldValueError),
    /// The section forbids this field.
    Forbidden,
    /// The section rejects fields it neither declares nor forbids.
    Unknown,
}

impl FieldError {
    /// The field's lowercase name, or `None` for an unknown field, whose name is untrusted.
    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// The position of the offending field in the original section, or `None` for a missing field, which has no position.
    pub fn position(&self) -> Option<usize> {
        self.position
    }

    /// What went wrong.
    pub fn kind(&self) -> FieldErrorKind {
        self.kind
    }

    pub(crate) fn duplicate(name: &'static str, position: usize, first: usize) -> Self {
        Self {
            name: Some(name),
            position: Some(position),
            kind: FieldErrorKind::Duplicate { first },
        }
    }

    pub(crate) fn invalid(name: &'static str, position: usize, error: FieldValueError) -> Self {
        Self {
            name: Some(name),
            position: Some(position),
            kind: FieldErrorKind::Invalid(error),
        }
    }

    pub(crate) fn unknown(position: usize) -> Self {
        Self {
            name: None,
            position: Some(position),
            kind: FieldErrorKind::Unknown,
        }
    }

    pub(crate) fn missing(name: &'static str) -> Self {
        Self {
            name: Some(name),
            position: None,
            kind: FieldErrorKind::Missing,
        }
    }

    pub(crate) fn forbidden(name: &'static str, position: usize) -> Self {
        Self {
            name: Some(name),
            position: Some(position),
            kind: FieldErrorKind::Forbidden,
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name.unwrap_or("unknown");
        let position = self.position.unwrap_or_default();
        match self.kind {
            FieldErrorKind::Duplicate { first } => write!(
                formatter,
                "duplicate {name} field at position {position}, first at {first}"
            ),
            FieldErrorKind::Missing => write!(formatter, "missing required {name} field"),
            FieldErrorKind::Invalid(error) => write!(
                formatter,
                "invalid {name} field at position {position}: {error}"
            ),
            FieldErrorKind::Forbidden => {
                write!(formatter, "forbidden {name} field at position {position}")
            }
            FieldErrorKind::Unknown => write!(formatter, "unexpected field at position {position}"),
        }
    }
}

impl std::error::Error for FieldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            FieldErrorKind::Invalid(error) => Some(error),
            _ => None,
        }
    }
}
