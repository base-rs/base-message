# Interpretation

## Requirement

A field section must be interpreted through a declaration: a struct deriving `FieldSection` whose members each declare one message field by its lowercase name, or flatten another section. Each field type declares its combination as one of the crate's sealed markers, `Singleton`, `List`, or `Separate`, and a declaration's container must match it, enforced at compile time: a singleton as `Option<T>`, or as `T` when required, and a list or separate field as `Vec<T>`.

Field types and sections may borrow from the field values: `FieldType` and `FieldSection` carry the lifetime of the values, and a section may have one lifetime parameter and no other generic parameters. Owned field types work with any lifetime.

Interpretation must:

- match names case-insensitively;
- treat any second instance of a singleton as an error naming both positions, even with an identical value, and even when the first value was invalid and ignored;
- combine list instances in wire order, splitting elements at commas outside quoted strings, removing optional whitespace, and skipping empty elements, as RFC 9110 section 5.6.1 describes, without allocating to split them; an instance that does not split is skipped as a whole when its field ignores invalid values;
- keep separate values whole and in order;
- apply each field type's invalid-value policy, error or ignore, unless the declaration overrides it, except that a required declaration always treats an invalid value as an error, and declaring one to ignore invalid values is a compile-time error;
- report a missing required field, without a position;
- treat a field the section forbids as an error, never as ignored or passed on;
- ignore fields the section neither declares nor forbids, without parsing them, unless the section rejects unknown fields. A section that flattens a section rejecting unknown fields rejects them too, judged against all of its declarations, and declaring such a section to ignore unknown fields is a compile-time error.

`partition` must return the fields a section does not take, borrowed and with their positions in the original section, in a collection the caller chooses, so that another section can interpret them. A field a static table classified as undeclared for one section must reach the remainder by name.

A static table's classifications must be typed by the section they were made for, so that passing one section's classification to another is a compile-time error, and each classified entry must carry the table's own name, so that a decoder cannot pair a classification with another name.

A name declared or forbidden both by a section and by a section it flattens, or by two sections it flattens, must be a compile-time error that identifies the name or the sections. A member with more than one declaration, and any other malformed declaration, must be a compile-time error. Errors must name the field in the section's own spelling and give its position in the original section, and must never include a value or the name of an unknown field.

## Rationale

The user's goal is a standard interpretation of fields, because that is where implementations go wrong: taking the first or last of a repeated singleton enables request smuggling, naive comma splitting breaks quoted values and `Set-Cookie`, and silently ignoring a framing field such as `Transfer-Encoding` is dangerous. The user preferred static declarations, and asked that applications interpret fields under their own policy, reparsing where needed, with everything they do not declare ignored rather than stored. The user asked for the remainder to be collected at the call site rather than declared on the struct, and agreed to forbidden fields. A prototype found that letting an application declare a field a flattened section also declares silently starved one of them, which is why that is a compile-time error.

An independent review before the first release found that an untyped static-table classification passed to the wrong section read values into the wrong declarations and ignored forbidden fields, the misreading this crate exists to prevent; the user chose to type classifications by section, at no runtime cost, over checking names at run time. The user chose to support borrowing values before the first release, because adding a lifetime later would break every field type, and copying each value costs an allocation. The user decided that a required field always treats an invalid value as an error, because ignoring the value of a field the section requires left only a misleading report that it was missing, and that a flattened section's rejection of unknown fields carries over, since dropping it silently weakened a strict section. A missing field has no position, because the position past the last field a layer saw could name an unrelated field of the original section.

## Scope

`FieldType`, the `combination` markers, `InvalidValue`, `FieldSection`, the derive in `base-message-macros`, `FieldError`, `FieldName`, `FieldRef`, `IntoField`, `OwnedFields`, `StaticTableEntry`, `static_table_entries`, and the value helpers `split_list`, `trim_whitespace`, and `ascii_text`.

## Validation

The tests in `tests/field_section.rs` cover each combination, duplicates, invalid-value policies and their overrides, required fields with invalid values, missing, forbidden, and unknown fields, strictness carried over from flattened sections, flattening, partitioning with positions across layers, borrowing sections, and static-table classification. `tests/field_value.rs` covers list splitting. The compile-fail cases under `tests/ui/`, run by `tests/diagnostics.rs`, cover every rejected declaration, mismatched containers, sealed combinations, generic parameters, a table passed to another section, and shadowing across flattened sections.
