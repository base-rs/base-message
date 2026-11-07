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
struct TwoNames {
    #[field("a")]
    #[field("b")]
    value: Option<One>,
}

#[derive(FieldSection)]
struct NameThenFlatten {
    #[field("x-id")]
    #[fields(flatten)]
    value: Option<One>,
}

#[derive(FieldSection)]
#[field("host")]
struct FieldOnStruct {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct RequiredIgnoresInvalid {
    #[field("host", invalid = "ignore")]
    host: One,
}

fn main() {}
