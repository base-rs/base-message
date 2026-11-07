// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::request_section::{DECLARATIONS, DECLARED_NAMES, RequestSection, STATIC_TABLE};
use crate::{BenchmarkConfig, FIELDS_PER_SECTION, Variant, Workload};
use base_benchmark::{CurrentProcess, Interval, ProcessRecord, ResourceSource, Sample};
use base_message::{FieldName, FieldRef, FieldSection, IntoField};
use serde::Serialize;
use std::hint::black_box;
use std::io;

/// The stable workload identity in every record.
pub(crate) const WORKLOAD: &str = "base-message/interpretation";

/// A typical browser request, as in the measurement `design/performance.md` records: mostly declared fields, some undeclared, in mixed case.
pub(crate) const REQUEST: [(&str, &str); FIELDS_PER_SECTION] = [
    ("Host", "example.com"),
    ("User-Agent", "Mozilla/5.0"),
    ("Accept", "text/html"),
    ("Accept-Language", "en"),
    ("Accept-Encoding", "gzip, br"),
    ("Referer", "https://example.com/"),
    ("Cookie", "a=1"),
    ("Sec-Fetch-Dest", "document"),
    ("Sec-Fetch-Mode", "navigate"),
    ("Sec-Ch-Ua", "x"),
    ("Upgrade-Insecure-Requests", "1"),
    ("Connection", "keep-alive"),
    ("If-None-Match", "\"x\""),
    ("Priority", "u=0"),
];

/// The parameters in every record: the configuration, and the fixed shape of the workload.
#[derive(Serialize)]
struct Parameters<'a> {
    #[serde(flatten)]
    config: &'a BenchmarkConfig,
    fields_per_section: usize,
    declarations: usize,
}

/// What interpreting the request produced: each declaration's value length, and the positions of the fields a partition did not take.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    lengths: [Option<usize>; DECLARATIONS],
    remainder: Vec<usize>,
}

impl Outcome {
    /// Equal signatures mean equal interpretations: the number of remaining fields, and an FNV-1a hash of the whole outcome.
    fn signature(&self) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        let lengths = self
            .lengths
            .iter()
            .map(|length| length.map_or(u64::MAX, |length| length as u64));
        for value in lengths.chain(self.remainder.iter().map(|&position| position as u64)) {
            for byte in value.to_le_bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
        format!("{}:{hash:016x}", self.remainder.len())
    }
}

pub(crate) fn run(config: &BenchmarkConfig) -> io::Result<ProcessRecord> {
    run_with(config, CurrentProcess)
}

/// Run a validated configuration, sampling resources from `usage`, which tests replace to exercise failures.
fn run_with(config: &BenchmarkConfig, usage: impl ResourceSource) -> io::Result<ProcessRecord> {
    let operations = config
        .operations()
        .expect("a validated configuration's operations fit in u64");
    let lowercase: Vec<String> = REQUEST
        .iter()
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    let text = text_fields();
    let indexed = indexed_fields(&lowercase);
    // Both variants must interpret the request exactly as expected before anything is timed.
    let expected = expected_outcome(config.workload);
    let sample =
        preflight(config.workload, &text, &indexed, &expected).and_then(|()| {
            match config.variant {
                Variant::Text => measure(config, &text, operations, usage),
                Variant::Indexed => measure(config, &indexed, operations, usage),
            }
        })?;
    let parameters = Parameters {
        config,
        fields_per_section: FIELDS_PER_SECTION,
        declarations: DECLARATIONS,
    };
    let parameters = serde_json::to_value(parameters).expect("parameters serialize");
    Ok(ProcessRecord::new(WORKLOAD, parameters)
        .with_signature(expected.signature())
        .with_sample(sample))
}

/// The request as an HTTP/1.1 parser passes it: names as text, as written.
fn text_fields() -> Vec<(&'static str, &'static [u8])> {
    REQUEST
        .iter()
        .map(|(name, value)| (*name, value.as_bytes()))
        .collect()
}

/// The request as an HTTP/2 or HTTP/3 decoder passes it: lowercase names, those in the static table classified by declaration id, and the rest as text.
fn indexed_fields(lowercase: &[String]) -> Vec<(FieldName<'_, RequestSection>, &'static [u8])> {
    lowercase
        .iter()
        .zip(REQUEST)
        .map(
            |(name, (_, value))| match DECLARED_NAMES.iter().position(|entry| entry == name) {
                Some(index) => (FieldName::Indexed(STATIC_TABLE[index]), value.as_bytes()),
                None => (FieldName::Text(name.as_str()), value.as_bytes()),
            },
        )
        .collect()
}

/// The outcome computed directly from the request, independently of `base-message`.
fn expected_outcome(workload: Workload) -> Outcome {
    let lengths = DECLARED_NAMES.map(|declared| {
        REQUEST
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(declared))
            .map(|(_, value)| value.len())
    });
    let remainder = match workload {
        Workload::Interpret => Vec::new(),
        Workload::Partition => (0..REQUEST.len())
            .filter(|&position| {
                !DECLARED_NAMES
                    .iter()
                    .any(|declared| REQUEST[position].0.eq_ignore_ascii_case(declared))
            })
            .collect(),
    };
    Outcome { lengths, remainder }
}

/// Interpret the fields once, untimed, for the preflight check.
fn outcome<'a, F: IntoField<'a, RequestSection> + Copy>(
    workload: Workload,
    fields: &[F],
) -> io::Result<Outcome> {
    let result = match workload {
        Workload::Interpret => {
            RequestSection::interpret(fields.iter().copied()).map(|section| (section, Vec::new()))
        }
        Workload::Partition => {
            RequestSection::partition::<_, Vec<FieldRef<'a>>>(fields.iter().copied())
        }
    };
    let (section, remainder) = result.map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("preflight interpretation failed: {error}"),
        )
    })?;
    Ok(Outcome {
        lengths: section.lengths(),
        remainder: remainder.iter().map(FieldRef::position).collect(),
    })
}

/// Check that both variants interpret the request as expected.
fn preflight(
    workload: Workload,
    text: &[(&str, &[u8])],
    indexed: &[(FieldName<'_, RequestSection>, &[u8])],
    expected: &Outcome,
) -> io::Result<()> {
    outcome(workload, text).and_then(|actual| check(Variant::Text, &actual, expected))?;
    outcome(workload, indexed).and_then(|actual| check(Variant::Indexed, &actual, expected))
}

fn check(variant: Variant, actual: &Outcome, expected: &Outcome) -> io::Result<()> {
    if actual != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "preflight mismatch for the {variant:?} variant: expected {expected:?}, found {actual:?}"
            ),
        ));
    }
    Ok(())
}

/// Time the configured workload over `fields`, which preflight has checked interpret successfully.
fn measure<'a, F: IntoField<'a, RequestSection> + Copy>(
    config: &BenchmarkConfig,
    fields: &[F],
    operations: u64,
    usage: impl ResourceSource,
) -> io::Result<Sample> {
    // The workload is chosen outside the loop, so that each timed operation is the interpretation alone, including dropping its result.
    match config.workload {
        Workload::Interpret => repeat(config, operations, usage, || {
            black_box(
                RequestSection::interpret(black_box(fields).iter().copied())
                    .expect("preflight interpreted this section"),
            );
        }),
        Workload::Partition => repeat(config, operations, usage, || {
            let partition: (RequestSection, Vec<FieldRef<'a>>) =
                RequestSection::partition(black_box(fields).iter().copied())
                    .expect("preflight partitioned this section");
            black_box(partition);
        }),
    }
}

fn repeat(
    config: &BenchmarkConfig,
    operations: u64,
    usage: impl ResourceSource,
    mut operation: impl FnMut(),
) -> io::Result<Sample> {
    for _ in 0..config.warmup {
        operation();
    }
    let interval = Interval::start_with(usage)?;
    for _ in 0..config.iterations {
        operation();
    }
    interval.finish(operations)
}

#[cfg(test)]
mod tests;
