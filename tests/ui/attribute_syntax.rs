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
struct NameNotString {
    #[field(host)]
    host: Option<One>,
}

#[derive(FieldSection)]
struct MissingComma {
    #[field("host" invalid = "ignore")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct OptionNotIdentifier {
    #[field("host", "ignore")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct UnknownOption {
    #[field("host", strict = "yes")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct MissingEquals {
    #[field("host", invalid "ignore")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct PolicyNotString {
    #[field("host", invalid = ignore)]
    host: Option<One>,
}

#[derive(FieldSection)]
struct UnknownPolicy {
    #[field("host", invalid = "maybe")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct FieldsNotIdentifier {
    #[fields("flatten")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct UnknownFieldsWord {
    #[fields(spread)]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields("reject")]
struct SectionKeyNotIdentifier {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(strict)]
struct UnknownSectionKey {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(unknown "reject")]
struct SectionMissingEquals {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(unknown = reject)]
struct SectionPolicyNotString {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(unknown = "keep")]
struct UnknownSectionPolicy {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(forbid = "te")]
struct ForbidNotParenthesized {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(forbid(te))]
struct ForbiddenNotString {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(unknown = "reject" forbid("te"))]
struct SectionMissingComma {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct PolicyTwice {
    #[field("host", invalid = "error", invalid = "ignore")]
    host: Option<One>,
}

#[derive(FieldSection)]
#[fields(unknown = "reject", unknown = "ignore")]
struct UnknownTwice {
    #[field("host")]
    host: Option<One>,
}

fn main() {}
