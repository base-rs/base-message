// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message::{FieldError, FieldNames, FieldSection, static_table_entries};

static INNER: FieldNames = FieldNames::new(&["date", "host"], &["upgrade"], &[], false);
static OTHER: FieldNames = FieldNames::new(&["accept"], &[], &[], false);
static STRICT: FieldNames = FieldNames::new(&[], &[], &[], true);
static OUTER: FieldNames = FieldNames::new(&["x-id"], &["te"], &[&INNER, &OTHER], false);
static DEEP: FieldNames = FieldNames::new(&[], &[], &[&OTHER, &STRICT], false);

#[test]
fn numbers_declarations_and_finds_forbidden_names_at_every_level() {
    // Evaluated at run time here, as the derive's constants evaluate them at compile time.
    assert_eq!(OUTER.count(), 4);
    let ids: Vec<Option<usize>> = ["x-id", "date", "host", "accept", "te", "missing"]
        .iter()
        .map(|name| OUTER.declaration(name))
        .collect();
    assert_eq!(ids, [Some(0), Some(1), Some(2), Some(3), None, None]);
    assert!(
        OUTER.forbids("te")
            && OUTER.forbids("upgrade")
            && !OUTER.forbids("host")
            && !OUTER.forbids("accept")
    );
    assert!(OUTER.names("upgrade") && OUTER.names("accept") && !OUTER.names("missing"));
    assert!(format!("{:?}", OUTER.clone()).contains("x-id"));
}

#[test]
fn finds_rejection_of_unknown_fields_at_any_level() {
    assert!(STRICT.rejects_unknown());
    assert!(DEEP.rejects_unknown());
    assert!(!OUTER.rejects_unknown());
}

#[test]
fn finds_names_shared_at_any_level() {
    let shares = |left: &FieldNames, right: &FieldNames| left.shares_names_with(right);
    assert!(shares(&INNER, &OUTER), "a declared name");
    assert!(
        shares(&FieldNames::new(&[], &["upgrade"], &[], false), &INNER),
        "a forbidden name"
    );
    assert!(
        shares(&DEEP, &OTHER),
        "a name declared by a flattened section"
    );
    assert!(!shares(&OTHER, &INNER));
    assert!(!shares(&STRICT, &OUTER));
    assert!(
        !shares(&OUTER, &FieldNames::new(&["unrelated"], &[], &[], false)),
        "every level searched, nothing shared"
    );
}

/// A section implemented by hand, with more declarations than a static table can number.
struct Large;

/// Declaration 65,536 is the only one named `x`.
static LARGE: [&str; 65_537] = {
    let mut names = [""; 65_537];
    names[65_536] = "x";
    names
};

impl FieldSection<'_> for Large {
    const NAMES: FieldNames = FieldNames::new(&LARGE, &[], &[], false);
    type Identity = Self;
    type Builder = ();

    fn builder() {}

    fn accept(_: &mut (), _: &str, _: &[u8], _: usize) -> Result<bool, FieldError> {
        Ok(false)
    }

    fn accept_declaration(_: &mut (), _: usize, _: &[u8], _: usize) -> Result<(), FieldError> {
        Ok(())
    }

    fn finish((): ()) -> Result<Self, FieldError> {
        Ok(Self)
    }
}

#[test]
fn static_tables_are_limited_to_sixteen_bit_ids() {
    assert!(Large::interpret([("x", "")]).is_ok());
    let result = std::panic::catch_unwind(|| static_table_entries::<Large, 1>(&["x"]));
    assert!(result.is_err());
}
