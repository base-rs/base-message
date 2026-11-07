// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

fn main() -> std::process::ExitCode {
    base_message_benchmark_interpretation::command(
        std::env::args().skip(1),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
}
