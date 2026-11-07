use base_message::{combination::{List, Separate, Singleton}, FieldSection, FieldType, FieldValueError};

struct One;

impl FieldType for One {
    type Combination = Singleton;
    fn parse(_: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self)
    }
}

struct Many;

impl FieldType for Many {
    type Combination = List;
    fn parse(_: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self)
    }
}

struct Apart;

impl FieldType for Apart {
    type Combination = Separate;
    fn parse(_: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self)
    }
}

#[derive(FieldSection)]
#[fields(forbid("te"))]
struct Inner {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct DeclaresInnerName {
    #[fields(flatten)]
    inner: Inner,
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct DeclaresInnerForbiddenName {
    #[fields(flatten)]
    inner: Inner,
    #[field("te")]
    te: Option<One>,
}

#[derive(FieldSection)]
struct FlattensTwice {
    #[fields(flatten)]
    first: Inner,
    #[fields(flatten)]
    second: Inner,
}

fn main() {}
