# Changelog

## Unreleased

- Added optional, additive marketplace-listing fields under `[metadata]` in
  `af-core.toml`: `summary` (one-line), `homepage`, and
  `[[metadata.maintainers]]` (structured `name`/`email`/`role`/`homepage`).
  These coexist with the existing `description`, `repository`, and
  `authors[]` fields; existing manifests parse unchanged. Schema mirror
  added to `schemas/af-core.schema.json`. No `af_version` bump (non-breaking
  addition).
- Added `docs/semver-policy.md` documenting MAJOR/MINOR/PATCH rules for the
  CLI surface, manifest `af_version`, report `schema_version`/`report_version`,
  per-core `version`, and `AF_*` error codes. Referenced from
  `CONTRIBUTING.md` and `docs/manifest-reference.md`.

## 0.2.0-rc.1 - 2026-05-25

- Added the fail-closed `af release check --json` production gate, release
  readiness report payload, GitHub release workflow, external CI evidence
  bundle, Docker digest evidence, and release artifact checksum validation. New
  additive error code: `AF_RELEASE_READINESS_BLOCKED`.
- Added reproducible build and first-10-minutes documentation, GitHub private
  vulnerability reporting policy, supported-version/deprecation policy, and an
  examples overview including the generated `vendor-aware-skeleton` reference.
- Added the opt-in `fpga-ip-core-v1` standards profile, `af core standards`
  check/export commands, generated checklist/compliance matrix artifacts,
  manifest `[standards]` evidence declarations, and `spdx-hbom` package output.
  New additive error codes: `AF_STANDARDS_ARTIFACT_ITEM_INVALID`,
  `AF_STANDARDS_COLLECT_COPY_FAILED`, `AF_STANDARDS_EXPORT_FORMAT_UNSUPPORTED`,
  `AF_STANDARDS_PROFILE_UNKNOWN`, and `AF_STANDARDS_SAFETY_DOMAIN_UNSUPPORTED`.
- `af core standards check` now fail-closes malformed standards artifacts with
  row-level `validation_status`/`artifact_validations`, and generated
  `spdx-hbom` packages include SHA-256 checksums for declared source files.
- `af core standards scaffold` now creates missing `fpga-ip-core-v1` evidence
  placeholders without overwriting local files, can append idempotent
  `[standards]`/`[[standards.artifacts]]` manifest declarations with
  `--declare`, and `af core standards check --strict` fail-closes selected rows
  when required external validators such as `xmllint`, `peakrdl`, or
  `verible-verilog-lint` are unavailable.
- Added standards evidence utilities for the FPGA/IP commercial baseline:
  `af core standards doctor`, `drift`, `spdx-audit`, `collect`, plus
  `af core regs scaffold/check`. `standards check` now emits
  `gates.commercial_baseline_ready` and local `tool_availability`, SPDX audits
  can be declared as item 21 evidence, and collected CI/package reports are
  linked into `[[standards.artifacts]]` idempotently.
- Strengthened the standards flow with `af ci init --standards`, install hints
  in `standards doctor`, opportunistic `xmllint`/`peakrdl` validation in
  `standards check --strict`, collection of simulation/formal reports when
  present, and a complete `examples/standards-ready-core` reference layout.
- `af wrapper generate --target ipxact` now emits an IEEE 1685-2022-style
  component skeleton with manifest interfaces, ports, file sets, and AccelFury
  vendor extensions instead of the old SPIRIT 1.5 skeleton.
- `af core new` now accepts opt-in `--standards-profile fpga-ip-core-v1` to
  create standards placeholders and manifest evidence declarations at scaffold
  time; `af core report` now includes an additive standards summary when
  `[standards]` is declared; and `spdx-hbom` output now includes present
  standards evidence artifacts plus commit, dirty-tree, tag, and tag-signature
  provenance.
- Manifest v0.4 now supports explicit `rtl.clocking = "none"` and
  `rtl.reset = "none"` for clockless/resetless atomic cores without fake
  clock/reset ports. New additive error codes: `AF_RTL_CLOCKING_MODE_INVALID`
  and `AF_RTL_RESET_MODE_INVALID`.
- Documented the alpha-readiness scope and gates: the supported CLI surface is
  the manifest-first loop (`doctor`, `self check`, `manifest validate`,
  `core check/lint/sim/report`, `wrapper generate`, `ci generate`), while timing
  closure, CDC/RDC signoff, vendor production bitstreams, and hardware
  programming remain staged or out of scope.
- Added production-readiness contract guidance, claims matrix, and CI/release
  gate expectations for promoting `af` as a CLI/toolchain without overstating
  timing, CDC/RDC, vendor bitstream, or hardware-ready claims.
- Docker smoke now installs Icarus Verilog (`iverilog`/`vvp`) and runs an Icarus
  lint/simulation path on the Verilog-2001 `af-reset-sync` example, while
  Verilator/Yosys/FuseSoC/LiteX smoke remains on `af-pdm-rx`.
- The CLI contract guard now compares error-code inventories from
  `crates/**/src/**/*.rs` on both sides of the diff, avoiding false additive
  warnings from test-only `AF_*` strings.
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
  consumers can dispatch on `command_payload.kind` without sniffing the schema.
  Documented in `docs/cli-reference.md`.
- M3 reproducibility metadata (`host_os`, `host_arch`, `environment_hash`,
  `af_version`) and stdout/stderr log artifacts referenced from each
  `CommandRecord.stdout_log` / `stderr_log` field are also documented.
- Decomposition: `commands::self_check` (~280 LOC) extracted from
  `crates/af-cli/src/main.rs`. Cumulative reduction since the audit cycle began:
  5,299 → 3,464 LOC (-35%).
- Dead code removed: `af_backend::ExecutedCommand` and the
  `BackendReport.commands_executed` field. Their stdout_log / stderr_log
  responsibility was migrated to `CommandRecord` in an earlier commit.
- `af compatibility check`, `af signoff plan`, and `af dependency graph` now run
  structural manifest+RTL inspection (via the new
  `af_core::load_validated_manifest`) on each core input; broken manifests or
  missing source files fail with `AF_CORE_CHECK_FAILED` (exit 2) instead of
  returning a misleading `"passed"` / `"planned"` report.
- `af project classify --from-spec <path>` returns `AF_COMPLEXITY_SPEC_EMPTY`
  (exit 2) when the spec is empty or whitespace-only, instead of silently
  defaulting to `simple-portable`.
- `af board list` (human output) prefixes each entry with `[VERIFIED]` or
  `[DRAFT]` based on its registry `exact_pinout_status`. JSON output is
  unchanged.
- `af wrapper generate --board <id>` adds a warning to `wrapper.warnings` when
  the board is not `verified_on_hardware` (placeholder pinout), or when the id
  is not in the registry.
- `af core report` `board_hardware_evidence` maturity row now enumerates each
  declared board with a `(draft)` / `(verified-or-unknown)` tag and appends a
  specific limitation listing the placeholder board ids. New
  `MaturityInputs.placeholder_boards` field carries this signal.
- `af core report` `docker_ci_cd_evidence` row is now fail-closed: it requires
  an attributable CI run record (commit_sha matching the current HEAD,
  `conclusion = "success"`, plus workflow_run_url / artifact_bundle /
  sha256sums) ingested via the new `af evidence ingest --kind ci-run`. A
  workflow file alone, stale evidence, or a non-success conclusion all keep the
  row `blocked` with a specific limitation. Closes
  `AF.TODO.CI-CURRENT-TREE-EVIDENCE-GATE`.
- `af evidence ingest --kind ci-run` accepts a JSON input describing a single
  GitHub Actions style run; the report writes the normalized record under
  `.af-build/reports/evidence/ci_run_report-*.json` and `core report` consumes
  it.
- `af core new` now generates AccelFury Source Available License v1.0 legal
  artifacts (`LICENSE`, `COMMERCIAL-LICENSE.md`, `NOTICE`) for reusable IP
  cores, and `af core check` fails closed on placeholder or mismatched legal
  policy.
- Added complexity-aware `af` project classes, v0.3 manifest fields, class-aware
  core/project scaffolds, offline architecture/resource/compatibility/
  constructor/signoff/dependency commands, and planned vendor backend scaffolds.

## 0.1.0 - 2026-04-27

- Initial production-grade template generation for `af_mod_add`.
- Added Rust workspace, Verilator flow, Deno checks, board skeletons, and
  documentation baseline.
