// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use super::*;
use base_benchmark::ResourceUsage;

fn config(workload: Workload, variant: Variant) -> BenchmarkConfig {
    BenchmarkConfig {
        workload,
        variant,
        iterations: 2,
        warmup: 1,
    }
}

#[test]
fn both_variants_interpret_the_request_as_expected() {
    let lowercase: Vec<String> = REQUEST
        .iter()
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    for workload in [Workload::Interpret, Workload::Partition] {
        let expected = expected_outcome(workload);
        assert_eq!(outcome(workload, &text_fields()).unwrap(), expected);
        assert_eq!(
            outcome(workload, &indexed_fields(&lowercase)).unwrap(),
            expected
        );
    }
    let partition = expected_outcome(Workload::Partition);
    assert_eq!(
        partition.remainder,
        [7, 8, 9, 10, 13],
        "the undeclared fields remain at their positions"
    );
    assert_eq!(partition.lengths.iter().flatten().count(), 9);
    // The indexed variant classifies every declared field by id, and passes the rest as lowercase text.
    let indexed = indexed_fields(&lowercase);
    assert_eq!(
        indexed
            .iter()
            .filter(|(name, _)| matches!(name, FieldName::Indexed(..)))
            .count(),
        9
    );
    assert!(
        indexed
            .iter()
            .all(|(name, _)| name.as_str() == name.as_str().to_ascii_lowercase())
    );
}

#[test]
fn signatures_distinguish_workloads_but_not_variants() {
    let interpret = run(&config(Workload::Interpret, Variant::Text)).unwrap();
    let indexed = run(&config(Workload::Interpret, Variant::Indexed)).unwrap();
    let partition = run(&config(Workload::Partition, Variant::Indexed)).unwrap();
    let signature =
        |record: &ProcessRecord| serde_json::to_value(record).unwrap()["signature"].clone();
    assert_eq!(signature(&interpret), signature(&indexed));
    assert_ne!(signature(&interpret), signature(&partition));
    assert!(signature(&partition).as_str().unwrap().starts_with("5:"));
}

#[test]
fn preflight_failures_are_errors() {
    let duplicate = [("Host", &b"a"[..]), ("Host", b"b")];
    let error = outcome(Workload::Interpret, &duplicate).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("duplicate host field"));
    let lowercase: Vec<String> = REQUEST
        .iter()
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    let indexed = indexed_fields(&lowercase);
    let expected = expected_outcome(Workload::Interpret);
    assert!(preflight(Workload::Interpret, &text_fields(), &indexed, &expected).is_ok());
    assert!(preflight(Workload::Interpret, &duplicate, &indexed, &expected).is_err());
    let expected = expected_outcome(Workload::Partition);
    let error = check(
        Variant::Indexed,
        &expected_outcome(Workload::Interpret),
        &expected,
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("Indexed"));
}

/// Fails the `n`th resource snapshot.
struct Failing(usize);

impl ResourceSource for Failing {
    fn snapshot(&mut self) -> io::Result<ResourceUsage> {
        if self.0 == 0 {
            return Err(io::Error::other("resource sample failed"));
        }
        self.0 -= 1;
        CurrentProcess.snapshot()
    }

    fn resolution_ns(&self) -> u64 {
        1000
    }
}

#[test]
fn resource_failures_produce_no_record() {
    for variant in [Variant::Text, Variant::Indexed] {
        for workload in [Workload::Interpret, Workload::Partition] {
            for failing in [0, 1] {
                assert!(run_with(&config(workload, variant), Failing(failing)).is_err());
            }
        }
    }
}
