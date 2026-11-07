// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

/// The lowercase field names a section declares and forbids, with those of the sections it flattens, and whether it rejects unknown fields, as a constant.
///
/// The derive checks at compile time that no name is declared or forbidden at two levels, so that no declaration silently never receives values, and [`static_table_entries`](crate::static_table_entries) uses these names to classify a protocol's static table. Declaration ids number a section's own declarations first, in order, then each flattened section's, in order.
#[derive(Clone, Copy, Debug)]
pub struct FieldNames {
    declared: &'static [&'static str],
    forbidden: &'static [&'static str],
    flattened: &'static [&'static FieldNames],
    rejects_unknown: bool,
}

impl FieldNames {
    /// The names of one section, as the derive generates them.
    #[doc(hidden)]
    pub const fn new(
        declared: &'static [&'static str],
        forbidden: &'static [&'static str],
        flattened: &'static [&'static FieldNames],
        rejects_unknown: bool,
    ) -> Self {
        Self {
            declared,
            forbidden,
            flattened,
            rejects_unknown,
        }
    }

    /// The number of declarations, including those of flattened sections.
    pub const fn count(&self) -> usize {
        let mut count = self.declared.len();
        let mut section = 0;
        while section < self.flattened.len() {
            count += self.flattened[section].count();
            section += 1;
        }
        count
    }

    /// The declaration id of a lowercase field name, or `None` when no declaration has it.
    pub const fn declaration(&self, name: &str) -> Option<usize> {
        let mut index = 0;
        while index < self.declared.len() {
            if equal(self.declared[index], name) {
                return Some(index);
            }
            index += 1;
        }
        let mut offset = self.declared.len();
        let mut section = 0;
        while section < self.flattened.len() {
            if let Some(index) = self.flattened[section].declaration(name) {
                return Some(offset + index);
            }
            offset += self.flattened[section].count();
            section += 1;
        }
        None
    }

    /// Whether this section, or a section it flattens, forbids a lowercase field name.
    pub const fn forbids(&self, name: &str) -> bool {
        if contains(self.forbidden, name) {
            return true;
        }
        let mut section = 0;
        while section < self.flattened.len() {
            if self.flattened[section].forbids(name) {
                return true;
            }
            section += 1;
        }
        false
    }

    /// Whether the section rejects fields it neither declares nor forbids: because it says so, or because a section it flattens does.
    pub const fn rejects_unknown(&self) -> bool {
        if self.rejects_unknown {
            return true;
        }
        let mut section = 0;
        while section < self.flattened.len() {
            if self.flattened[section].rejects_unknown() {
                return true;
            }
            section += 1;
        }
        false
    }

    /// Whether this section, at any level, declares or forbids a lowercase field name.
    #[doc(hidden)]
    pub const fn names(&self, name: &str) -> bool {
        self.declaration(name).is_some() || self.forbids(name)
    }

    /// Whether this section, at any level, declares or forbids a name that `other` also does.
    #[doc(hidden)]
    pub const fn shares_names_with(&self, other: &FieldNames) -> bool {
        let mut index = 0;
        while index < self.declared.len() {
            if other.names(self.declared[index]) {
                return true;
            }
            index += 1;
        }
        let mut index = 0;
        while index < self.forbidden.len() {
            if other.names(self.forbidden[index]) {
                return true;
            }
            index += 1;
        }
        let mut section = 0;
        while section < self.flattened.len() {
            if self.flattened[section].shares_names_with(other) {
                return true;
            }
            section += 1;
        }
        false
    }
}

const fn contains(names: &[&str], name: &str) -> bool {
    let mut index = 0;
    while index < names.len() {
        if equal(names[index], name) {
            return true;
        }
        index += 1;
    }
    false
}

const fn equal(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
