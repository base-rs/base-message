// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message_benchmark_interpretation::{BenchmarkConfig, FIELDS_PER_SECTION, command, run};
use std::io::{self, Write};
use std::process::{Command, ExitCode};

const BINARY: &str = env!("CARGO_BIN_EXE_base-message-benchmark-interpretation");

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

#[test]
fn all_options_are_recorded_and_invalid_options_fail() {
    let values = args(&[
        "--workload",
        "partition",
        "--variant",
        "indexed",
        "--iterations",
        "2",
        "--warmup",
        "0",
    ]);
    let config = BenchmarkConfig::parse(values.clone()).unwrap();
    assert_eq!(config.operations(), Some(2 * FIELDS_PER_SECTION as u64));
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    assert_eq!(command(values, &mut stdout, &mut stderr), ExitCode::SUCCESS);
    assert!(stderr.is_empty());
    let record: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(record["parameters"]["workload"], "partition");
    assert_eq!(record["parameters"]["variant"], "indexed");
    assert_eq!(record["parameters"]["iterations"], 2);
    assert_eq!(
        record["parameters"]["fields_per_section"],
        FIELDS_PER_SECTION
    );
    assert_eq!(record["parameters"]["declarations"], 40);
    for input in [
        vec!["--workload", "wrong"],
        vec!["--variant", "wrong"],
        vec!["--iterations", "0"],
        vec!["--iterations", "not-an-integer"],
        vec!["--warmup", "-1"],
        vec!["--unknown", "1"],
        vec!["--warmup"],
        vec!["--warmup", "1", "--warmup", "2"],
    ] {
        stdout.clear();
        stderr.clear();
        assert_eq!(
            command(args(&input), &mut stdout, &mut stderr),
            ExitCode::FAILURE,
            "{input:?}"
        );
        assert!(stdout.is_empty());
        assert!(!stderr.is_empty());
    }
    let overflowing = BenchmarkConfig {
        iterations: u64::MAX,
        ..BenchmarkConfig::default()
    };
    assert_eq!(overflowing.operations(), None);
    assert!(run(&overflowing).is_err());
    assert!(
        run(&BenchmarkConfig {
            iterations: 0,
            ..BenchmarkConfig::default()
        })
        .is_err()
    );
}

struct RejectOutput;

impl Write for RejectOutput {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("output failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn output_errors_and_diagnostic_errors_do_not_change_failure_status() {
    let mut errors = Vec::new();
    assert_eq!(
        command(
            args(&["--iterations", "1", "--warmup", "0"]),
            &mut RejectOutput,
            &mut errors
        ),
        ExitCode::FAILURE
    );
    assert_eq!(
        command(
            args(&["--invalid", "1"]),
            &mut Vec::new(),
            &mut RejectOutput
        ),
        ExitCode::FAILURE
    );
    RejectOutput.flush().unwrap();
}

#[test]
fn binary_emits_one_record_or_exits_unsuccessfully() {
    for workload in ["interpret", "partition"] {
        for variant in ["text", "indexed"] {
            let output = Command::new(BINARY)
                .args([
                    "--workload",
                    workload,
                    "--variant",
                    variant,
                    "--iterations",
                    "1",
                    "--warmup",
                    "0",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 1);
            let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        }
    }
    let output = Command::new(BINARY).arg("--bad").output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

/// The shared runner accepts this workload's records for every configured scenario, and checks that both variants did identical work.
#[test]
fn shared_runner_smokes_every_scenario() {
    use base_benchmark::Workspace;
    let workspace = Workspace::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let package = Some("base-message-benchmark-interpretation");
    let scenarios = workspace.suite(package).unwrap().scenarios;
    let directory = tempfile::tempdir().unwrap();
    let campaigns = workspace
        .smoke_executable(
            package,
            std::path::Path::new(BINARY),
            &directory.path().join("smoke"),
            Some(directory.path().join("lock")),
        )
        .unwrap();
    assert_eq!(campaigns.len(), scenarios.len());
    assert!(!campaigns.is_empty());
    for (campaign, scenario) in campaigns.iter().zip(&scenarios) {
        assert_eq!(campaign.summary.scenario, scenario.name);
        assert_eq!(campaign.summary.subjects.len(), 2, "{}", scenario.name);
        assert!(campaign.report.starts_with("smoke:"));
    }
}
