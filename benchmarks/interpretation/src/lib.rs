// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Workloads for interpreting a field section with `base-message`, by name and by static-table id.
//!
//! Each process emits one [`base_benchmark::ProcessRecord`]. Collection, rounds, controls, and summaries come from `base-benchmark`.

mod benchmark_config;
mod request_section;
mod workloads;

pub use benchmark_config::{BenchmarkConfig, Variant, Workload};

use base_benchmark::ProcessRecord;
use std::io::{self, Write};
use std::process::ExitCode;

/// The fields in the measured request. Each is one measured operation, so per-operation figures are per field.
pub const FIELDS_PER_SECTION: usize = 14;

/// Run one configured workload. Setup, the correctness preflight of both variants, and warm-up are excluded from measurement. A preflight mismatch is an `InvalidData` error and produces no record.
pub fn run(config: &BenchmarkConfig) -> io::Result<ProcessRecord> {
    config.validate()?;
    workloads::run(config)
}

/// Execute the CLI, emitting exactly one record on success and none on failure.
pub fn command(
    arguments: impl IntoIterator<Item = String>,
    out: &mut impl Write,
    errors: &mut impl Write,
) -> ExitCode {
    let result = BenchmarkConfig::parse(arguments)
        .and_then(|config| run(&config).and_then(|record| record.emit(out)));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Reporting failure cannot turn a failed benchmark into a successful measurement.
            let _ = writeln!(errors, "benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}
