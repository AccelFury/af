# Changelog

## Unreleased

- `af registry check --json` now includes advisory `catalog_readiness` for
  fpga.chat v1 export blockers, including missing board revisions/source
  locators and non-OSI core licenses, without turning structural registry
  validity failures into catalog-policy failures.
- `af manifest validate af-core.toml` now resolves same-workspace core
  dependencies the same way when invoked from inside a core directory as
  `af manifest validate projects/<core>/af-core.toml` does from the workspace
  root. Closes `AF.TODO.MANIFEST-VALIDATE-CWD-PARITY`.
- Manifest v0.3 now supports optional FIFO and reset semantic contracts,
  same-workspace dependency paths with parameter overrides, stricter
  manifest-vs-RTL port-width checking, `af compatibility` stream FIFO adapter
  suggestions, generic `[[contracts.protocols]]` adapter hints, and
  `af wrapper generate --target stream-fifo`.
- M3 typed report contract is now uniform across **every** command family:
  `af core check`, `af core lint`, `af core sim`, `af core formal`,
  `af core package`, `af core report`, `af core tooling`, `af build`,
  `af doctor`, and `af flash` all emit a `command_payload` block with a
  `kind`-discriminated union (`check` / `lint` / `simulation` / `formal` /
  `build` / `package` / `report` / `tooling` / `doctor` / `flash`). LLM/CI
  consumers can dispatch on `command_payload.kind` without sniffing the
  schema. Documented in `docs/cli-reference.md`.
- M3 reproducibility metadata (`host_os`, `host_arch`, `environment_hash`,
  `af_version`) and stdout/stderr log artifacts referenced from each
  `CommandRecord.stdout_log` / `stderr_log` field are also documented.
- Decomposition: `commands::self_check` (~280 LOC) extracted from
  `crates/af-cli/src/main.rs`. Cumulative reduction since the audit cycle
  began: 5,299 → 3,464 LOC (-35%).
- Dead code removed: `af_backend::ExecutedCommand` and the
  `BackendReport.commands_executed` field. Their stdout_log / stderr_log
  responsibility was migrated to `CommandRecord` in an earlier commit.
- `af compatibility check`, `af signoff plan`, and `af dependency graph` now run structural manifest+RTL inspection (via the new `af_core::load_validated_manifest`) on each core input; broken manifests or missing source files fail with `AF_CORE_CHECK_FAILED` (exit 2) instead of returning a misleading `"passed"` / `"planned"` report.
- `af project classify --from-spec <path>` returns `AF_COMPLEXITY_SPEC_EMPTY` (exit 2) when the spec is empty or whitespace-only, instead of silently defaulting to `simple-portable`.
- `af board list` (human output) prefixes each entry with `[VERIFIED]` or `[DRAFT]` based on its registry `exact_pinout_status`. JSON output is unchanged.
- `af wrapper generate --board <id>` adds a warning to `wrapper.warnings` when the board is not `verified_on_hardware` (placeholder pinout), or when the id is not in the registry.
- `af core report` `board_hardware_evidence` maturity row now enumerates each declared board with a `(draft)` / `(verified-or-unknown)` tag and appends a specific limitation listing the placeholder board ids. New `MaturityInputs.placeholder_boards` field carries this signal.
- `af core report` `docker_ci_cd_evidence` row is now fail-closed: it requires an attributable CI run record (commit_sha matching the current HEAD, `conclusion = "success"`, plus workflow_run_url / artifact_bundle / sha256sums) ingested via the new `af evidence ingest --kind ci-run`. A workflow file alone, stale evidence, or a non-success conclusion all keep the row `blocked` with a specific limitation. Closes `AF.TODO.CI-CURRENT-TREE-EVIDENCE-GATE`.
- `af evidence ingest --kind ci-run` accepts a JSON input describing a single GitHub Actions style run; the report writes the normalized record under `.af-build/reports/evidence/ci_run_report-*.json` and `core report` consumes it.
- `af core new` now generates AccelFury Source Available License v1.0 legal artifacts (`LICENSE`, `COMMERCIAL-LICENSE.md`, `NOTICE`) for reusable IP cores, and `af core check` fails closed on placeholder or mismatched legal policy.
- Added complexity-aware `af` project classes, v0.3 manifest fields, class-aware
  core/project scaffolds, offline architecture/resource/compatibility/
  constructor/signoff/dependency commands, and planned vendor backend scaffolds.

## 0.1.0 - 2026-04-27

- Initial production-grade template generation for `af_mod_add`.
- Added Rust workspace, Verilator flow, Deno checks, board skeletons, and
  documentation baseline.
