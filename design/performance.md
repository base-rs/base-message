# Performance

## Requirement

Interpreting a section must not hash field names or allocate for fields the section does not take. The derive must generate name dispatch at compile time, branching on a name's length and first byte before comparing the few declarations that share both, so that the cost per field does not grow with the number of declarations. Values must be stored directly in the section's struct, with no intermediate map.

Interpretation must also accept fields already classified by a protocol's static table, such as HPACK's or QPACK's, through declaration ids that `static_table_entries` computes at compile time, so that an indexed field is dispatched without comparing names. This indexed path must be available before the first release.

## Rationale

The user asked whether comparing names costs more than hashing them. A measurement on an AMD Ryzen 9 9950X3D, with 40 declared HTTP field names and a 14-field browser request, found per-field dispatch costs of 12.2 ns for a hash table of lowercased names, 9.5 ns for comparing each declaration in turn, and 5.7 ns for a jump on length and first byte. Interpreting the whole section through the prototype derive cost 4.7 to 5.0 ns per field by name and 2.6 to 2.9 ns with static-table ids. The user considers the indexed path important for HTTP/2 and HTTP/3 speed. HTTP/2 and HTTP/3 have different static tables, so each protocol crate maps its own table; this crate supplies only the declaration ids and the compile-time classification.

## Scope

The code `base-message-macros` generates, and `FieldSection::interpret`, `FieldSection::partition`, `FieldNames`, `FieldName`, and `static_table_entries`.

## Validation

Inspect the derive's generated `accept` for a `match` on length and first byte, and `accept_declaration` for a `match` on declaration id. A benchmark package, to be added before the first release, measures interpretation by name and by static-table id against the figures above; a change that makes either measurably slower needs a recorded reason.
