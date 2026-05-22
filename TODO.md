# TODO - AccelFury `af`

Active lifecycle-tool gaps discovered while hardening generated cores.

- [fpga.chat catalog] AF.TODO.BOARDS-REVISION-CAPTURE — `registries/boards.registry.json`
  has 30 board rows; all of them carry `exact_pinout_status = "draft_placeholder"`
  and none declare a board `revision` field. fpga.chat v1 `BoardRecord` schema
  (`schemas/board-record.schema.json`) makes `revision` a REQUIRED top-level
  string. Until upstream captures revisions, every AccelFury board row is
  deferred at `apps/web/public/data/catalog-deferred.json` with reason
  `revision_missing_from_upstream`. Action requested: add `revision` (and a
  `revision_source_locator`) to each board row, sourced from the official board
  schematic or product page; verify against the documented board revision
  printed on silkscreen or in the schematic title block. Tracking: `af registry
  check --json` reports these rows under `catalog_readiness.board_records`.
- [fpga.chat catalog] AF.TODO.CORES-OSI-LICENSE — `registries/cores.registry.json`
  and all `examples/*/af-core.toml` files declare
  `license = "AccelFury Source Available License v1.0"`. This is NOT on the
  OSI-approved list, so per `.claude/agents/catalog-curator.md` "License gate"
  the AccelFury cores cannot enter the public fpga.chat catalog v1. Either
  publish AccelFury cores under an OSI-approved license (Apache-2.0, BSD-3-Clause,
  MIT, MPL-2.0, etc.) for the entries intended to be shareable, or leave them
  deferred. fpga.chat will mirror the upstream license verbatim and will not
  paraphrase or upgrade it. Tracking: `af registry check --json` reports these
  entries under `catalog_readiness.core_licenses`.


- AF.TODO.CLOCKLESS-NORESET-CORE-MANIFESTS — `af` v0.3 manifests currently force at least one `[[clocks]]`, one `[[resets]]` and one `[[ports]]`, and `af core check` requires declared clocks/resets to bind to RTL ports. This blocks honest generic portable cores that are purely combinational (`af-mux-tree`, `af-adder-tree`) or clocked without reset (`af-ram-1r1w`, `af-ram-tdp`, `af-rom`) unless authors add fake reset/clock ports or downgrade to `af_version = "0.1"`. Impact: reusable atomic cores either grow misleading protocol glue or lose v0.3 contract features. Required behavior: v0.3 should support `rtl.clocking = "none"` / `rtl.reset = "none"` or explicit `clockless = true` / `resetless = true`, skip clock/reset binding checks only for those declared modes, keep port and width checks active, and report adapter hints when a composite shell expects clock/reset semantics. Verification: add manifest parser tests, rtl-inspector tests for clockless and resetless cores, compatibility adapter diagnostics, and a wrapper-generation refusal test that fails closed instead of fabricating dummy ports.

## Recently closed

- AF.TODO.MANIFEST-VALIDATE-CWD-PARITY — `af manifest validate af-core.toml`
  now resolves the core directory as `.` when invoked from inside a core
  directory, so same-workspace dependency resolution matches
  `af manifest validate projects/<core>/af-core.toml` from the workspace root.
  Covered by `manifest_validate_resolves_workspace_dependencies_from_core_cwd`.
- AF.TODO.GENERIC-PROTOCOL-CONTRACTS — v0.3 contracts now include
  `[[contracts.protocols]]` for non-FIFO reusable protocol semantics.
  `af compatibility check` consumes these contracts for protocol, width, clock,
  reset, and adapter-hint diagnostics while RTL generation remains limited to
  known wrapper targets such as `stream-fifo`.
- AF.TODO.WORKSPACE-LOCAL-DEPS — v0.3 `[[dependencies.cores]]` entries now
  support same-workspace `path` plus `parameter_overrides`; `af manifest
  validate`, `af core check`, and `af core report` resolve sibling cores
  without requiring `deps/` symlinked RTL source paths, keep arbitrary
  dependency paths fail-closed at the workspace boundary, and attribute
  dependency manifests/sources to the owning core in report artifacts. Tests in
  `crates/af-core`, `crates/af-cli` coverage through command JSON/report
  generation.
- AF.TODO.CI-CURRENT-TREE-EVIDENCE-GATE — `docker_ci_cd_evidence` now demotes
  workflow-file-only state to `planned` unless a current-tree run record and a
  `SHA256SUMS` bundle are present in the artifact list. Tests in
  `crates/af-report/src/lib.rs`.
