# `base-message-benchmark-interpretation`

A private workspace package that measures interpreting a field section with `base-message`, by name and by static-table id, as [`design/performance.md`](../../design/performance.md) requires. No library depends on this package.

The workload binary measures one configuration and writes one [`base-benchmark`](https://github.com/base-rs/base-benchmark) process record. Preparation, randomized rounds, layout-replica controls, locking, validation, and summaries come from that shared tooling through the `benchmark:*` Bake tasks.

## Quick validation

```sh
cargo test -p base-message-benchmark-interpretation --locked
cargo bake benchmark:smoke
```

Smoke runs build the current tree and run every scenario once with tiny iteration counts. They check correctness and lifecycle only. The binary exits unsuccessfully, without a record, on invalid options, resource errors, output failures, or a preflight mismatch.

## Workload

The section declares 40 common HTTP field names, each an optional singleton whose field type keeps only its value's length, so that the workload measures dispatch and accumulation rather than value parsing. The input is a 14-field browser request, of which 9 fields are declared and 5 are not. Both match the measurement recorded in the design document.

| Variant | Input |
| --- | --- |
| `text` | Names as text, in mixed case, as an HTTP/1.1 parser passes them. The baseline of each comparison. |
| `indexed` | Lowercase names. Those in the static table arrive classified by declaration id, as an HTTP/2 or HTTP/3 decoder passes them, and the rest as text. The table is a stand-in holding the 40 declared names; a real table, such as HPACK's, would also hold names the section does not declare. |

| Workload | Operation |
| --- | --- |
| `interpret` | `FieldSection::interpret`, discarding undeclared fields. |
| `partition` | `FieldSection::partition` into a `Vec<FieldRef>`, which allocates and fills the remainder of 5 fields. |

Each measured operation is one field: a section of 14 fields counts as 14 operations, so per-operation figures are per field. Every timed iteration interprets the whole request and drops the result, keeping the whole section alive through `black_box`, as code reading the section would. The prototype harness behind the design's figures read one field instead, which let the compiler discard the others, so its figures are lower and the two harnesses' figures are not comparable; [pull request #4](https://github.com/base-rs/base-message/pull/4) describes a one-off comparison of the two.

Before timing, every process interprets the request with both variants and compares each declaration's value length, and for `partition` the remainder's positions, with an outcome computed directly from the request. Input preparation, this preflight, and warm-up are excluded from measurement. The record's signature is a hash of the expected outcome, so both variants of a scenario must report the same one.

## Parameters

All options use `--name value`; defaults are defined in [`BenchmarkConfig`](src/benchmark_config.rs).

- `--workload`: `interpret` or `partition`.
- `--variant`: `text` or `indexed`.
- `--iterations`: measured sections, positive. The default is 5,000,000, or 70,000,000 fields.
- `--warmup`: sections interpreted before measurement; may be zero.

Each record reports `wall_time`, `cpu_time`, `user_cpu_time`, `system_cpu_time`, and both context-switch counts for the measured operations, with `fields_per_section` and `declarations` in its parameters.

## Scenarios and comparisons

[`Cargo.toml`](Cargo.toml) declares the `interpret` and `partition` scenarios, each with the variants `text` and `indexed`. Run `cargo bake benchmark:list` to see the resolved configuration.

```sh
cargo bake benchmark:prepare --tree current=. --replicas 1
cargo bake benchmark:run interpret --artifacts target/base-benchmark/prepared-TIME/artifacts.json --cpus 2
```

To compare revisions, check them out into separate directories, for example with `git worktree add`, and pass each with `--tree LABEL=PATH`, baseline first. Recorded runs (`--mode recorded`) also require at least 10 rounds, explicit `--cpus`, a `--decision-rule`, `--idle-host true`, committed trees, and a layout replica when comparing separately built trees; see the [`base-benchmark` usage guide](https://github.com/base-rs/base-benchmark/blob/main/context/usage.md). For a reviewed measurement, keep the campaign directory under `results/<topic>/` with a readme describing it. No results are recorded yet.
