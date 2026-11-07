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
struct SeparateInOption {
    #[field("set-cookie")]
    cookie: Option<Apart>,
}

#[derive(FieldSection)]
struct ListRequired {
    #[field("accept")]
    accept: Many,
}

#[derive(FieldSection)]
struct SingletonInVec {
    #[field("host")]
    hosts: Vec<One>,
}

#[derive(FieldSection)]
struct NotAFieldType {
    #[field("host")]
    host: &'static str,
}

fn main() {}
