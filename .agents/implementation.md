# Repository Guidance

Read `readme.md` and the guides in `context/` for the crates' purpose and usage. Before planning or changing behavior, discover and read applicable `design/*.md`, especially `design/scope.md` before adding public items. Use `base-design-review` to assess plans and changes, and `base-design-author` when user requirements are introduced, changed, or removed. Review design compliance before opening or updating a pull request.

Install shared guidance with `cargo bake agent:context:install`; discover it with `cargo bake agent:context:list` and `cargo bake agent:context:show`. Keep private task tooling out of the libraries' dependency graphs.

The derive's compile errors are snapshot-tested under `tests/ui/`. After changing a diagnostic, regenerate the snapshots with `TRYBUILD=overwrite cargo test --test diagnostics` and review every changed `.stderr` file.

Use `base-security-audit` for security reviews and record them in `security/audit-*.md`. Use `base-security-advisory` for vulnerability reports and potentially advisory-worthy findings. Read relevant `// SECURITY:` comments and `design/*.md` before reporting a security finding.
