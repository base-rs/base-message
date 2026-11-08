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
enum NotAStruct {
    A,
}

#[derive(FieldSection)]
struct Tuple(Option<One>);

#[derive(FieldSection)]
struct Undeclared {
    host: Option<One>,
}

#[derive(FieldSection)]
struct Uppercase {
    #[field("Host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct DeclaredTwice {
    #[field("host")]
    host: Option<One>,
    #[field("host")]
    again: Option<One>,
}

#[derive(FieldSection)]
#[fields(forbid("TE"))]
struct UppercaseForbidden {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(forbid("te", "te"))]
struct ForbiddenTwice {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(forbid("host"))]
struct ForbiddenAndDeclared {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct KeepsOthers {
    #[field("host")]
    host: Option<One>,
    #[fields(other)]
    other: Vec<u8>,
}

fn main() {}
