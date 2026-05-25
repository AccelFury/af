# Production Readiness

`af` production readiness means the CLI/toolchain contract is stable enough for
CI and automation. It does not mean FPGA timing closure, CDC/RDC signoff, vendor
production bitstreams, board bring-up, or hardware programming are supported
without separate evidence.

Boundary: `vendor production bitstreams` are not claimed without separate
evidence.

## Supported Production Contract

The production-supported surface is the manifest-first loop documented in
`docs/cli-reference.md`: `doctor`, `self check`, `manifest validate`,
`core check`, `core lint`, `core sim`, `core report`, `wrapper generate`, and
`ci generate`, plus `release check` for the repository release gate.

For that surface, production releases must preserve:

- documented CLI flags and argument meaning;
- `--json` report and error-envelope shape;
- exit-code bands from `docs/cli-reference.md`;
- `AF_*` error-code compatibility, with removals treated as breaking;
- `af-core.toml` `af_version` compatibility and migration notes;
- `AfReport` `schema_version` / `report_version` compatibility;
- semver release notes for every additive or breaking contract change.

## Required Gates

Production candidates must pass:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`;
- `.claude/skills/af-cli-contract-guard/check.sh`;
- host CLI smoke for the production-supported manifest-first commands;
- Docker smoke through `make smoke`;
- artifact checksum generation with `SHA256SUMS`;
- `af release check --json` with a clean source tree, exact-run CI evidence,
  release artifact checksums, Docker digest/smoke evidence, and docs claim
  audit.

External release evidence must include the CI run URL, commit SHA, success
conclusion, uploaded artifact bundle, and checksums. A workflow file alone is
not production evidence.

## Claims Matrix

| Claim                  | Production status                             | Required evidence                                   |
| ---------------------- | --------------------------------------------- | --------------------------------------------------- |
| CLI contract stability | Supported when release gates pass             | Contract guard, tests, docs, changelog              |
| Manifest validation    | Supported for documented `af_version` values  | Parser tests and CLI smoke                          |
| JSON/error contract    | Supported for production commands             | Envelope tests and schema snapshots                 |
| Open-source smoke      | Supported through Docker runtime              | `make smoke` artifacts and checksums                |
| FuseSoC export         | Supported as deterministic wrapper generation | Generated `.core` artifact                          |
| LiteX support          | Skeleton/reference only                       | Generated skeleton plus limitation                  |
| Timing closure         | Not claimed                                   | Vendor timing report required                       |
| CDC/RDC signoff        | Not claimed                                   | Dedicated CDC/RDC evidence required                 |
| Vendor bitstream       | Not claimed                                   | Vendor build and bitstream evidence required        |
| Hardware-ready         | Not claimed                                   | Board/reference evidence and bring-up logs required |

## Support Discipline

Production releases must keep `README.md`, `docs/known-limitations.md`,
`docs/release-process.md`, the PR template, generated reports, and this document
aligned. Unsupported claims must be downgraded to limitations or blockers.

Troubleshooting guidance starts with `af doctor --json`, then the command report
under `--build-root`, then Docker smoke. Security and audit readiness are local
toolchain checks; `af doctor --json` reports whether `deno task audit:repo` is
declared without executing write-capable audit tasks.

Deprecations must be documented before removal. Removing a command, flag, JSON
field, manifest field, schema property, exit-code meaning, or `AF_*` error code
requires a breaking-change note and the appropriate version bump.
