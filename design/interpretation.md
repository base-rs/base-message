# Interpretation

## Requirement

A field section must be interpreted through a declaration: a struct deriving `FieldSection` whose fields each declare one message field by its lowercase name, or flatten another section. Each field type declares its combination as a type-level marker, and a declaration's container must match it, enforced at compile time: a singleton as `Option<T>`, or as `T` when required, and a list or separate field as `Vec<T>`.

Interpretation must:

- match names case-insensitively;
- treat any second instance of a singleton as an error naming both positions, even with an identical value, and even when the first value was invalid and ignored;
- combine list instances in wire order, splitting elements at commas outside quoted strings, removing optional whitespace, and skipping empty elements, as RFC 9110 section 5.6.1 describes;
- keep separate values whole and in order;
- apply each field type's invalid-value policy, error or ignore, unless the declaration overrides it;
- report a missing required field;
- treat a field the section forbids as an error, never as ignored or passed on;
- ignore fields the section neither declares nor forbids, without parsing or storing them, unless the section rejects unknown fields.

`partition` must return the fields a section does not take, borrowed and with their positions in the original section, in a collection the caller chooses, so that another section can interpret them. A field a static table classified as undeclared for one section must reach the remainder as text.

A name declared or forbidden both by a section and by a section it flattens must be a compile-time error. Errors must name the field in the section's own spelling and give its position in the original section, and must never include a value or the name of an unknown field.

## Rationale

The user's goal is a standard interpretation of fields, because that is where implementations go wrong: taking the first or last of a repeated singleton enables request smuggling, naive comma splitting breaks quoted values and `Set-Cookie`, and silently ignoring a framing field such as `Transfer-Encoding` is dangerous. The user preferred static declarations, and asked that applications interpret fields under their own policy, reparsing where needed, with everything they do not declare ignored rather than stored. The user asked for the remainder to be collected at the call site rather than declared on the struct, and agreed to forbidden fields. A prototype found that letting an application declare a field a flattened section also declares silently starved one of them, which is why that is a compile-time error.

## Scope

`FieldType`, the `combination` markers, `InvalidValue`, `FieldSection`, the derive in `base-message-macros`, `FieldError`, `FieldName`, `FieldRef`, `OwnedFields`, and the value helpers `split_list`, `trim_whitespace`, and `ascii_text`.

## Validation

The tests in `tests/field_section.rs` cover each combination, duplicates, invalid-value policies and their overrides, missing, forbidden, and unknown fields, flattening, partitioning with positions across layers, and static-table classification. `tests/field_value.rs` covers list splitting. The compile-fail cases under `tests/ui/`, run by `tests/diagnostics.rs`, cover every rejected declaration, mismatched containers, and shadowing across flattened sections.
