# Releases

## v0.1.0

- Add `FieldType`, whose combination marker, `Singleton`, `List`, or `Separate`, decides how a field's instances combine, and `#[derive(FieldSection)]`, which generates the interpretation of a header or trailer section into a struct: duplicate singletons are errors, lists respect quoted commas, invalid values follow each field's policy, forbidden fields are refused, and undeclared fields are ignored or rejected.
- Field types and sections can borrow their values from the input, through the lifetime on `FieldType<'a>` and one lifetime parameter on a section.
- Add `FieldSection::partition`, which returns the fields a section does not take, borrowed and with their original positions, so that layers interpret one message's fields in turn.
- Add `static_table_entries`, `StaticTableEntry`, and `FieldName`, so that HTTP/2 and HTTP/3 decoders interpret fields from their static tables by declaration id, without comparing names. Entries are typed by the section they were classified for, so one section's table cannot be used with another.
