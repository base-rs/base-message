// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::fmt;

/// Why a field value is invalid, as a fixed description. It never includes the value, which may be large or untrusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldValueError {
    reason: &'static str,
}

impl FieldValueError {
    /// An error with a fixed description, such as `"not a decimal length"`.
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }

    /// The description.
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for FieldValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)
    }
}

impl std::error::Error for FieldValueError {}
