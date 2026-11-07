// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Items the derive's generated code uses. They are not part of the public API and may change in any release.

use crate::FieldError;

pub use crate::accumulate::{Accumulate, Slot};

/// One field as a section receives it, whatever form it arrived in.
pub struct IncomingField<'a> {
    pub(crate) position: usize,
    pub(crate) name: IncomingName<'a>,
    pub(crate) value: &'a [u8],
}

/// A field's name as a section receives it: as text, or classified by a static table built for the section.
#[derive(Clone, Copy)]
pub enum IncomingName<'a> {
    Text(&'a str),
    Declaration(u16),
    Forbidden(&'static str),
    Undeclared(&'static str),
}

/// A required field is absent.
pub fn missing(name: &'static str) -> FieldError {
    FieldError::missing(name)
}

/// The section forbids a field it received by name.
pub fn forbidden(name: &'static str, position: usize) -> FieldError {
    FieldError::forbidden(name, position)
}
