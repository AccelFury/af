# TODO - AccelFury `af`

## Production-ready blockers

Local production-readiness hardening for `af` as a CLI/toolchain is now staged
in this workspace. Timing closure, CDC/RDC signoff, vendor production
bitstreams, and hardware-ready status remain separate evidence-gated claims.

### Closed locally in this workspace

- [x] AF.PROD.STABLE-CONTRACT - Documented the production-supported contract for
      `af-core.toml`, supported CLI commands, JSON/error envelopes, exit codes,
      schema/report versioning, and semver compatibility rules in
      `docs/cli-reference.md`, `docs/release-process.md`, and
      `docs/production-readiness.md`.
- [x] AF.PROD.LOCAL-GATES - Hardened the repository gates around `fmt`,
      `clippy`, `cargo test`, `.claude/skills/af-cli-contract-guard/check.sh`,
      host CLI smoke, Docker smoke, and checksum generation.
- [x] AF.PROD.SMOKE-MATRIX-LOCAL - Extended local regression coverage for
      Verilator, Icarus, Yosys, FuseSoC, LiteX skeleton, Docker flow, broken
      manifests, missing tools, bad paths, backend-unavailable states, and
      production docs/workflow invariants.
- [x] AF.PROD.CONTRACT-GUARD-NOISE - Fixed contract guard error-code scanning so
      current and base snapshots both scan `crates/**/src/**/*.rs`, avoiding
      false additive warnings from test-only `AF_*` strings.
- [x] AF.PROD.CLAIMS-MATRIX - Added a production claims matrix that separates
      CLI/toolchain readiness from timing, CDC/RDC, vendor bitstream,
      board-ready, hardware-ready, and security-certification claims.

### Remaining external or owner-gated actions

- [ ] AF.PROD.CI-EVIDENCE - Capture a successful external CI run for the exact
      release commit. Environment blocker: requires GitHub Actions or equivalent
      CI with Docker access. Verification: CI run URL, commit SHA, success
      conclusion, uploaded artifact bundle, and `SHA256SUMS`.
- [ ] AF.PROD.RELEASE-ARTIFACTS - Publish release artifacts. Environment
      blocker: requires repository release permissions and release-owner
      approval. Verification: signed or checksummed Git tag, release notes,
      binary artifacts, reproducible build instructions, and linked
      `SHA256SUMS`.
- [ ] AF.PROD.DOCKER-PUBLISH - Publish the Docker image outside the local
      machine. Environment blocker: requires registry credentials and
      project-owner image naming policy. Verification: immutable image digest
      and release notes linking the digest.
- [ ] AF.PROD.SUPPORT-OWNER-POLICY - Finalize support/security ownership.
      Project-owner blocker: choose security contact, support SLA/non-SLA
      wording, deprecation windows, and audit cadence. Verification:
      README/release-process links plus a reviewed security/support policy.
- [ ] AF.PROD.SIGNOFF-CLAIMS - Add separate evidence before claiming timing
      closure, CDC/RDC signoff, vendor production bitstreams, board-ready
      status, or hardware-ready status. Evidence blocker: requires vendor tools,
      board hardware where applicable, captured reports/logs, and reviewed
      limitations.

## Active lifecycle-tool gaps

Active lifecycle-tool gaps discovered while hardening generated cores.

- [fpga.chat catalog] AF.TODO.BOARDS-REVISION-CAPTURE —
  `registries/boards.registry.json` has 30 board rows; all of them carry
  `exact_pinout_status = "draft_placeholder"` and none declare a board
  `revision` field. fpga.chat v1 `BoardRecord` schema
  (`schemas/board-record.schema.json`) makes `revision` a REQUIRED top-level
  string. Until upstream captures revisions, every AccelFury board row is
  deferred at `apps/web/public/data/catalog-deferred.json` with reason
  `revision_missing_from_upstream`. Action requested: add `revision` (and a
  `revision_source_locator`) to each board row, sourced from the official board
  schematic or product page; verify against the documented board revision
  printed on silkscreen or in the schematic title block. Tracking:
  `af registry
  check --json` reports these rows under
  `catalog_readiness.board_records`.
- [fpga.chat catalog] AF.TODO.CORES-OSI-LICENSE —
  `registries/cores.registry.json` and all `examples/*/af-core.toml` files
  declare `license = "AccelFury Source Available License v1.0"`. This is NOT on
  the OSI-approved list, so per `.claude/agents/catalog-curator.md` "License
  gate" the AccelFury cores cannot enter the public fpga.chat catalog v1. Either
  publish AccelFury cores under an OSI-approved license (Apache-2.0,
  BSD-3-Clause, MIT, MPL-2.0, etc.) for the entries intended to be shareable, or
  leave them deferred. fpga.chat will mirror the upstream license verbatim and
  will not paraphrase or upgrade it. Tracking: `af registry check --json`
  reports these entries under `catalog_readiness.core_licenses`.

## Recently closed

- AF.TODO.CLOCKLESS-NORESET-CORE-MANIFESTS — v0.4 manifests now accept
  `rtl.clocking = "none"` and `rtl.reset = "none"` for clockless/resetless
  atomic cores. Clock/reset array requirements are skipped only for those
  explicit modes; port and width checks remain active.
- AF.TODO.MANIFEST-VALIDATE-CWD-PARITY — `af manifest validate af-core.toml` now
  resolves the core directory as `.` when invoked from inside a core directory,
  so same-workspace dependency resolution matches
  `af manifest validate projects/<core>/af-core.toml` from the workspace root.
  Covered by `manifest_validate_resolves_workspace_dependencies_from_core_cwd`.
- AF.TODO.GENERIC-PROTOCOL-CONTRACTS — v0.3 contracts now include
  `[[contracts.protocols]]` for non-FIFO reusable protocol semantics.
  `af compatibility check` consumes these contracts for protocol, width, clock,
  reset, and adapter-hint diagnostics while RTL generation remains limited to
  known wrapper targets such as `stream-fifo`.
- AF.TODO.WORKSPACE-LOCAL-DEPS — v0.3 `[[dependencies.cores]]` entries now
  support same-workspace `path` plus `parameter_overrides`;
  `af manifest
  validate`, `af core check`, and `af core report` resolve
  sibling cores without requiring `deps/` symlinked RTL source paths, keep
  arbitrary dependency paths fail-closed at the workspace boundary, and
  attribute dependency manifests/sources to the owning core in report artifacts.
  Tests in `crates/af-core`, `crates/af-cli` coverage through command
  JSON/report generation.
- AF.TODO.CI-CURRENT-TREE-EVIDENCE-GATE — `docker_ci_cd_evidence` now demotes
  workflow-file-only state to `planned` unless a current-tree run record and a
  `SHA256SUMS` bundle are present in the artifact list. Tests in
  `crates/af-report/src/lib.rs`.
