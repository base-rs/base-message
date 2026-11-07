// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

/// The lowercase field names a section declares and forbids, with those of the sections it flattens, as constants.
///
/// They let the derive reject a name declared at two levels at compile time, rather than one declaration silently never receiving values, and let [`static_table_entries`](crate::static_table_entries) classify a protocol's static table at compile time. Declaration ids number a section's own declarations first, in order, then each flattened section's, in order.
#[derive(Debug)]
pub struct FieldNames {
    /// The names this section declares.
    pub own: &'static [&'static str],
    /// The names this section forbids.
    pub forbidden: &'static [&'static str],
    /// The names of the sections this section flattens, in order.
    pub flattened: &'static [&'static FieldNames],
}

impl FieldNames {
    /// The number of declarations, including those of flattened sections.
    pub const fn count(&self) -> usize {
        let mut count = self.own.len();
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
        while index < self.own.len() {
            if equal(self.own[index], name) {
                return Some(index);
            }
            index += 1;
        }
        let mut offset = self.own.len();
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

    /// The section's own spelling of a forbidden name, at any level, matched case-insensitively, so an error never borrows from the input.
    pub(crate) fn forbidden_name(&self, name: &str) -> Option<&'static str> {
        self.forbidden
            .iter()
            .copied()
            .find(|forbidden| forbidden.eq_ignore_ascii_case(name))
            .or_else(|| {
                self.flattened
                    .iter()
                    .find_map(|section| section.forbidden_name(name))
            })
    }

    /// Whether any name is declared or forbidden more than once, at any level.
    pub const fn conflicts(&self) -> bool {
        let local = self.own.len() + self.forbidden.len();
        let mut index = 0;
        while index < local {
            let name = self.local(index);
            let mut other = index + 1;
            while other < local {
                if equal(name, self.local(other)) {
                    return true;
                }
                other += 1;
            }
            let mut section = 0;
            while section < self.flattened.len() {
                if self.flattened[section].names(name) {
                    return true;
                }
                section += 1;
            }
            index += 1;
        }
        let mut section = 0;
        while section < self.flattened.len() {
            if self.flattened[section].conflicts() {
                return true;
            }
            let mut other = section + 1;
            while other < self.flattened.len() {
                if self.flattened[section].shares_names_with(self.flattened[other]) {
                    return true;
                }
                other += 1;
            }
            section += 1;
        }
        false
    }

    /// This section's own declared names, then its own forbidden names.
    const fn local(&self, index: usize) -> &'static str {
        if index < self.own.len() {
            self.own[index]
        } else {
            self.forbidden[index - self.own.len()]
        }
    }

    /// Whether this section, at any level, declares or forbids a name.
    const fn names(&self, name: &str) -> bool {
        self.declaration(name).is_some() || self.forbids(name)
    }

    /// Whether this section, at any level, declares or forbids a name that `other` also does.
    const fn shares_names_with(&self, other: &FieldNames) -> bool {
        let mut index = 0;
        while index < self.own.len() + self.forbidden.len() {
            if other.names(self.local(index)) {
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
