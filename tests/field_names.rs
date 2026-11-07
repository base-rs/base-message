// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message::{FieldNames, StaticTableEntry, static_table_entries};

static INNER: FieldNames = FieldNames {
    own: &["date", "host"],
    forbidden: &["upgrade"],
    flattened: &[],
};
static OTHER: FieldNames = FieldNames {
    own: &["accept"],
    forbidden: &[],
    flattened: &[],
};
static OUTER: FieldNames = FieldNames {
    own: &["x-id"],
    forbidden: &["te"],
    flattened: &[&INNER, &OTHER],
};

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
    let table = static_table_entries(&OUTER, &["host", "upgrade", "user-agent", "accept"]);
    assert_eq!(
        table,
        [
            StaticTableEntry::Declaration(2),
            StaticTableEntry::Forbidden,
            StaticTableEntry::Undeclared,
            StaticTableEntry::Declaration(3)
        ]
    );
}

static TWICE: FieldNames = FieldNames {
    own: &["a", "a"],
    forbidden: &[],
    flattened: &[],
};
static DECLARED_AND_FORBIDDEN: FieldNames = FieldNames {
    own: &["a"],
    forbidden: &["a"],
    flattened: &[],
};
static DECLARES_INNER: FieldNames = FieldNames {
    own: &["date"],
    forbidden: &[],
    flattened: &[&INNER],
};
static FORBIDS_INNER: FieldNames = FieldNames {
    own: &[],
    forbidden: &["upgrade"],
    flattened: &[&INNER],
};
static FLATTENS_TWICE: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&INNER, &INNER],
};
static FLATTENS_CONFLICT: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&TWICE],
};
static WRAPS_OTHER: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&OTHER],
};
static SHARES_DEEPLY: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&OTHER, &WRAPS_OTHER],
};
static SHARES_FROM_DEEPER: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&WRAPS_OTHER, &OTHER],
};
static UNRELATED_THEN_SHARED: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&INNER, &WRAPS_OTHER],
};
static SHARES_AFTER_UNRELATED: FieldNames = FieldNames {
    own: &[],
    forbidden: &[],
    flattened: &[&UNRELATED_THEN_SHARED, &OTHER],
};

#[test]
fn finds_names_declared_or_forbidden_twice() {
    let cases: [(&FieldNames, bool); 11] = [
        (&OUTER, false),
        (&TWICE, true),
        (&DECLARED_AND_FORBIDDEN, true),
        (&DECLARES_INNER, true),
        (&FORBIDS_INNER, true),
        (&FLATTENS_TWICE, true),
        (&FLATTENS_CONFLICT, true),
        (&SHARES_DEEPLY, true),
        (&SHARES_FROM_DEEPER, true),
        (&UNRELATED_THEN_SHARED, false),
        (&SHARES_AFTER_UNRELATED, true),
    ];
    for (index, (names, conflicts)) in cases.into_iter().enumerate() {
        assert_eq!(names.conflicts(), conflicts, "case {index}");
    }
    assert!(format!("{OUTER:?}").contains("x-id"));
}

#[test]
fn static_tables_are_limited_to_sixteen_bit_ids() {
    let names: Vec<&'static str> = (0..=usize::from(u16::MAX) + 1)
        .map(|index| &*Box::leak(format!("f{index}").into_boxed_str()))
        .collect();
    let last = names[names.len() - 1];
    let large = FieldNames {
        own: Box::leak(names.into_boxed_slice()),
        forbidden: &[],
        flattened: &[],
    };
    let result = std::panic::catch_unwind(|| static_table_entries(&large, &[last]));
    assert!(result.is_err());
}
