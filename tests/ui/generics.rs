// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message::{combination::{List, Separate, Singleton}, FieldSection, FieldType, FieldValueError};

struct One;

impl FieldType<'_> for One {
    type Combination = Singleton;
    fn parse(_: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self)
    }
}

struct Many;

impl FieldType<'_> for Many {
    type Combination = List;
    fn parse(_: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self)
    }
}

struct Apart;

impl FieldType<'_> for Apart {
    type Combination = Separate;
    fn parse(_: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self)
    }
}

#[derive(FieldSection)]
struct TypeParameter<T> {
    #[field("host")]
    host: Option<T>,
}

#[derive(FieldSection)]
struct TwoLifetimes<'a, 'b> {
    #[field("host")]
    host: Option<&'a &'b One>,
}

#[derive(FieldSection)]
struct BoundedLifetime<'a: 'static> {
    #[field("host")]
    host: Option<&'a One>,
}

#[derive(FieldSection)]
struct WhereClause<'a> where One: Sized {
    #[field("host")]
    host: Option<&'a One>,
}

fn main() {}
