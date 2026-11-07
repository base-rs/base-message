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

use base_message::{FieldName, StaticTableEntry, static_table_entries};

#[derive(FieldSection)]
struct Request {
    #[field("host")]
    host: Option<One>,
}

#[derive(FieldSection)]
struct Other {
    #[field("content-length")]
    length: Option<One>,
}

const TABLE: [StaticTableEntry<Other>; 1] = static_table_entries::<Other, 1>(&["content-length"]);

fn interpret_with_another_sections_table() {
    let _ = Request::interpret([(TABLE[0], &b"5"[..])]);
    let _ = Request::interpret([(FieldName::Indexed(TABLE[0]), &b"5"[..])]);
}

fn main() {}
