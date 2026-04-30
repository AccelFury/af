# AccelFury `af` Compliance Review And Development Roadmap

Date: 2026-04-30

Scope: compare the current `af` repository state against the expanded
AccelFury IP Toolchain MVP-0/MVP-1 technical specification, then define a
quality, usefulness, and functional-maturity roadmap.

## Executive Summary

The repository satisfies the earlier narrow MVP-0/MVP-1 core: Rust workspace,
`af-core.toml` parsing, manifest-first core checks, Verilator availability and
lint/smoke command construction, FuseSoC `.core` generation, JSON/Markdown
reports, GitHub Actions, broken fixtures, and the `examples/af-pdm-rx` example.

It does not yet satisfy the expanded architecture/specification as written. The
largest gaps are LiteX, Yosys, SymbiYosys, openFPGALoader, vendor backend crates,
artifact-store/log contracts, full expanded CLI surface, v0.1 nested manifest
schema shape, port-to-RTL matching, clock/reset binding to ports, report JSON
contracts, devcontainer, and several documentation deliverables.

Current status: **MVP-0/1 baseline mostly pass; expanded MVP-1/MVP-2
architecture partial**.

## Verified Current State

Observed CLI commands:

- `af doctor`
- `af manifest validate`
- `af core check`
- `af core new`
- `af core lint`
- `af core sim`
- `af core report`
- `af registry check`
- `af board matrix`
- `af board new`
- `af vectors generate`
- `af wrapper generate`
- `af ci generate`

Observed workspace crates:

- `af-cli`
- `af-core`
- `af-manifest`
- `af-rtl-inspector`
- `af-backend`
- `af-backend-verilator`
- `af-backend-fusesoc`
- `af-report`
- `af-board-db`
- `af-wrapper-gen`
- `af-security`
- `af-ci`
- extra imported/support crates: `af-field-ref`, `af-host`, `af-vectors`

Observed backend crates missing from expanded TЗ:

- `af-backend-litex`
- `af-backend-yosys`
- `af-backend-sby`
- `af-backend-vendor`
- `af-backend-flash`

## Architecture Compliance

| Requirement | Status | Evidence | Gap |
| --- | --- | --- | --- |
| `af CLI` orchestrates services | Pass | `crates/af-cli` calls manifest, core, backends, board DB, reports, CI | CLI still contains some scaffolding/writer logic |
| Manifest parser/schema validator | Partial | `af-manifest` typed TOML parser and validation | Does not match expanded nested `schema_version/kind/[name]` schema |
| RTL inspector | Partial | checks source/include/testbench existence and top text presence | Missing port declaration matching and clock/reset port binding |
| Board database | Partial | registry validation plus `af-board.toml` compatibility profiles | Expanded `af-board.toml` schema not implemented |
| Wrapper generator | Partial | FuseSoC target works | LiteX skeleton target missing |
| Backend planner | Partial | simple `BuildPlan` exists | Missing typed target/capability/prepared-run model |
| Secure command runner | Partial | no-shell `Command`, path traversal rejection | No allowlist, env policy, timeout, redaction, offline policy |
| Artifact store | Partial | reports and wrappers written under build root | No first-class artifact-store crate/contract |
| Report engine | Partial | JSON/Markdown report writer exists | Does not implement simulation/build report contracts |
| CI/doc generator | Partial | GitHub Actions generator exists | No devcontainer; no GitLab generator |

## Backend Compliance

| Backend | Status | Notes |
| --- | --- | --- |
| Verilator | Partial | probes `verilator --version`; lint uses `--lint-only`; sim is currently a lint-only smoke check, not `--cc --exe --build` |
| FuseSoC | Partial | deterministic `.core` generation exists; `doctor` probes `fusesoc --version`; no `fusesoc core list` validation path |
| LiteX | Gap | no backend crate, wrapper skeleton, build dry-run, or report path |
| Yosys | Gap | no syntax/synthesis smoke backend |
| SymbiYosys | Gap | no `.sby` backend or `core formal` command |
| openFPGALoader | Gap | no flash backend |
| Vendor tools | Gap | no Gowin/Vivado/Quartus/Radiant/Diamond backend crates |

## CLI Compliance

| Command | Status | Current Equivalent |
| --- | --- | --- |
| `af init` | Gap | `af core new` exists but not workspace/core `af init` UX |
| `af core check` | Pass | implemented |
| `af core sim` | Partial | Verilator smoke is lint-only |
| `af core lint` | Pass/Partial | Verilator lint implemented; exit code taxonomy incomplete |
| `af core formal` | Gap | no command |
| `af core package` | Gap | no tar/manifest package command |
| `af core report` | Pass/Partial | reports exist; contract is simpler than spec |
| `af board list` | Gap | no command |
| `af board check` | Gap | no direct `af-board.toml` command |
| `af build` | Gap | no LiteX/Gowin build command |
| `af flash` | Gap | no openFPGALoader command |
| `af clean` | Gap | no artifact clean command |
| `af doctor` | Pass/Partial | probes `af`, Verilator, FuseSoC; not Python/Deno/vendor EULA |
| `af backend list` | Gap | no command |
| `af backend run` | Gap | no replay/debug command |
| `af manifest validate` | Pass/Partial | validates current schema, not expanded schema |
| `af wrapper generate` | Partial | FuseSoC only; no LiteX skeleton |
| `af ci generate` | Partial | GitHub Actions only |

Current global flags `--json`, `--verbose`, `--quiet`, and `--build-root` are
present.

Exit-code compliance is partial. Current implementation uses success plus broad
validation/backend/write codes, but not the full expanded 0..12 taxonomy
(`simulation failed`, `lint failed`, `formal failed`, `security policy
violation`, `artifact/report missing`, etc.).

## Manifest Schema Compliance

Current `af-core.toml` shape is a flat schema:

- root `af_version`, `name`, `vendor`, `library`, `core`, `version`
- `[metadata]`
- `[rtl]`
- `[sources]`
- arrays for `parameters`, `ports`, `clocks`, `resets`, `interfaces`,
  `testbenches`
- optional vectors/tooling/formal/boards/backend compatibility/limitations

Expanded TЗ requires a different v0.1 contract:

- `schema_version = "0.1"`
- `kind = "accelfury.core"`
- `[name]` table with `vendor/library/core/version`
- `[[sources]]` entries with `path/file_type/role`
- `[[include_dirs]]`
- richer `parameters`, `ports`, `clocks`, `resets`, `stream_interfaces`,
  `csr`, `interrupts`, `formal`, `boards`, `backend_compatibility`, and
  structured `known_limitations`

Status: **partial, incompatible shape**. The parser aliases `schema_version` to
`af_version`, but it does not parse `kind`, nested `[name]`, source entries,
string parameter widths, generated-source role policy, stream interfaces, CSR,
interrupts, or migration commands.

## RTL Inspector Compliance

| Inspector Requirement | Status |
| --- | --- |
| Source files exist | Pass |
| Include dirs exist | Partial, warning-only |
| Testbench files exist | Pass |
| Top module text appears | Pass |
| Manifest ports appear in module declaration | Gap |
| Clock/reset policy: each clock/reset bound to a port | Gap |
| Verilator `--lint-only` external diagnostics | Pass/Partial |
| `lint_report.json` with warnings/errors | Partial, backend report exists but no dedicated contract |

## Report Compliance

Current reports contain:

- `generated_by`
- report version
- status
- core identity
- tool versions
- commands
- artifacts
- warnings
- limitations

Gaps against expanded contracts:

- no `schema_version`/`kind` per report type;
- no dedicated `simulation_report.json` or `build_report.json` contract;
- stdout/stderr are currently in command records, not log file artifact paths;
- no duration, started timestamp, environment hash, bitstream hash, resource
  summary, timing summary, or reproducibility block;
- `commands` do not include env allowlist or timeout policy.

## Security Compliance

| Requirement | Status | Notes |
| --- | --- | --- |
| No shell interpolation by default | Pass | uses `std::process::Command` with program/args |
| Path traversal protection | Pass | relative path normalizer rejects `..` and absolute paths |
| Allowlisted executables from toolchain policy | Gap | no `af-toolchain.toml` support |
| User scripts disabled unless explicitly allowed | Gap | no script execution policy model |
| Offline mode/no hidden network | Partial | no known hidden network in Rust path, but no policy enforcement |
| Tool versions captured | Partial | captured for `af`, Verilator, FuseSoC paths |
| Commands logged with argv/cwd/env allowlist | Partial | argv/cwd logged, env not modeled |
| Secret redaction | Gap | no redaction filter |
| Vendor EULA/license warning | Gap | no vendor probing/EULA model |

## Documentation Compliance

Present:

- `README.md`
- `docs/architecture.md`
- `docs/manifest-reference.md`
- `docs/cli-reference.md`
- `docs/core-author-guide.md`
- `docs/security-model.md`
- `examples/af-pdm-rx/README.md`
- template/provenance/licensing/agent docs

Missing or incomplete relative to expanded DR list:

- `docs/product-requirements.md`
- `docs/software-requirements-specification.md`
- `docs/technical-design.md`
- `docs/backend-author-guide.md`
- `docs/board-author-guide.md`
- `docs/testing-strategy.md`
- `docs/release-process.md`
- `docs/known-limitations.md`
- `docs/roadmap.md`
- `docs/adr/0001-rust-over-python-backends.md`
- `docs/adr/0002-manifest-first-design.md`
- `docs/adr/0003-no-lifecycle-ownership-of-vendor-tools.md`
- `docs/adr/0004-generated-vs-handwritten-rtl.md`

## `af-pdm-rx` Compliance

Current `af-pdm-rx` explicitly excludes PDM-to-PCM conversion and passes current
core checks.

Expanded PDM acceptance gaps:

- no raw grouped PDM word output;
- no valid/ready stream interface;
- no `sample_ready_i`;
- no `WORD_BITS` parameter;
- testbench is a structural smoke file, not reset/valid-ready/grouping behavior
  verification;
- reports do not yet include an audio-quality warning specific to PCM absence.

This is acceptable for the earlier MVP boundary, but not for the expanded
`AC-PDM-001..007` contract.

## Roadmap

### Phase 0: Stabilize The Current Baseline

Goal: keep the existing MVP trustworthy while the expanded design is added.

1. Freeze the current command contracts in `docs/cli-reference.md`.
2. Add an automated compliance smoke script that runs:
   - `af doctor --json`
   - `af manifest validate examples/af-pdm-rx/af-core.toml --json`
   - `af core check examples/af-pdm-rx --json`
   - `af core lint examples/af-pdm-rx --backend verilator --json`
   - `af wrapper generate examples/af-pdm-rx --target fusesoc --json`
3. Add tests proving every CLI error payload has `code/message/hint/exit_code`.
4. Add tests for generated-header presence across `.core`, CI, Markdown reports,
   and future wrapper files.
5. Keep generated hardware policy explicit: wrappers/build scripts only; no
   generated CDC/FIFO/filter/bridge logic.

Exit criteria:

- Rust fmt/clippy/test pass.
- Broken fixtures produce structured errors.
- Current `main` stays releasable.

### Phase 1: Align Schemas With The Expanded TЗ

Goal: support the specified v0.1 manifest shape without breaking current files.

1. Implement `schema_version` and `kind` as first-class fields.
2. Add a compatibility parser for both current flat manifests and expanded
   nested `[name]` manifests.
3. Add typed models for:
   - `[[sources]] path/file_type/role`
   - `[[include_dirs]]`
   - parameter width references
   - port `kind`, `clock_domain`, interface tags
   - clock `port`
   - reset `port/style/clock_domain`
   - stream interfaces
   - CSR and interrupts
   - structured backend compatibility
   - structured known limitations
4. Add `af manifest migrate --from 0.1 --to 0.2` with non-overwrite default.
5. Add schema fixtures for current, expanded, and migration cases.

Exit criteria:

- Expanded example manifest parses and validates.
- Current manifests still parse.
- Migration writes adjacent files unless `--write` is passed.

### Phase 2: Complete Manifest-First RTL Inspection

Goal: satisfy the MVP inspector contract without writing a full SV parser.

1. Implement shallow module-declaration extraction for current top module.
2. Match manifest port names against the top declaration text.
3. Validate that clock/reset manifest entries reference existing ports.
4. Validate `clock_domain` references.
5. Validate generated source roles: generated files cannot appear as
   handwritten RTL without explicit generated role.
6. Write dedicated `core_check_report.json` and `lint_report.json` artifacts.

Exit criteria:

- Missing port, missing reset port, unknown clock domain, and generated-source
  policy fixtures fail deterministically.
- `af core check examples/af-pdm-rx --json` reports per-check pass/fail fields.

### Phase 3: Backend Contract Upgrade

Goal: make backends pluggable without changing `af-cli`.

1. Replace the simple backend trait with the expanded contract:
   - `BackendId`
   - `ToolInfo`
   - `BackendCapability`
   - `BuildPlan`
   - `BackendTarget`
   - `PreparedRun`
   - `ExecutedCommand`
   - diagnostics and metrics
2. Move command execution logs to files under build root.
3. Add env allowlist, timeout, and duration recording.
4. Add `af backend list`.
5. Add `af backend run` to replay prepared plans for debugging.
6. Split exit codes for lint, simulation, backend unavailable, backend failure,
   security policy, and artifact/report missing.

Exit criteria:

- Backends can register capabilities without modifying CLI match arms.
- All external commands produce stdout/stderr log artifacts.

### Phase 4: LiteX Reference Wrapper MVP

Goal: satisfy the expanded MVP LiteX skeleton requirement while preserving the
handwritten-RTL rule.

1. Add `crates/af-backend-litex`.
2. Add `wrapper generate --target litex --board <board>`.
3. Generate only skeleton/top/build files marked with the AccelFury generated
   header.
4. Add Tang Nano 20K primary and Tang Primer 20K experimental wrapper fixtures.
5. Add JSON/Markdown build dry-run reports.

Exit criteria:

- Generated LiteX skeleton is deterministic and golden-tested.
- No generated CDC/FIFO/audio/bus-bridge RTL is produced.

### Phase 5: Reports And Artifact Store

Goal: make outputs useful for CI, audit, and engineering review.

1. Add first-class artifact store model.
2. Implement report contracts:
   - `core_check_report.json`
   - `lint_report.json`
   - `simulation_report.json`
   - `build_report.json`
3. Include timestamps, duration, tool versions, commands, artifacts, warnings,
   limitations, and reproducibility metadata.
4. Add secret redaction before report writes.
5. Add Markdown equivalents for each report.

Exit criteria:

- Every command that performs work writes JSON and Markdown reports.
- Reports reference command log artifacts instead of large inline logs.

### Phase 6: Board And Toolchain Maturity

Goal: make board support useful without false confidence.

1. Implement expanded `af-board.toml`.
2. Add `af board list`.
3. Add `af board check <path-or-id>`.
4. Add `af-toolchain.toml` and offline policy support.
5. Probe Python, Deno, LiteX, Yosys, SBY, openFPGALoader, and vendor tools in
   `af doctor`.
6. Record vendor EULA/license status as warnings.
7. Add devcontainer for the open-source flow.

Exit criteria:

- Tang Nano 20K and Tang Primer 20K profiles validate under expanded schema.
- Missing optional tools are structured warnings, not CI failures.

### Phase 7: Yosys, SBY, Flash, And Vendor Roadmap

Goal: stage post-MVP backend value without bloating the MVP.

1. Add `af-backend-yosys` syntax/synthesis smoke.
2. Add `af core formal` and `af-backend-sby` for small checks.
3. Add `af-backend-flash` and `af flash` once bitstream artifacts exist.
4. Add Gowin project/Tcl generator first.
5. Delay Vivado/Quartus/Radiant/Diamond until the open-source flow is stable.

Exit criteria:

- Each backend has probe/prepare/run tests with fake command runner coverage.
- Vendor backends are optional and never required in default CI.

### Phase 8: `af-pdm-rx` Functional Maturity

Goal: decide whether `af-pdm-rx` remains raw bit capture or becomes the expanded
raw grouped stream core.

1. If keeping current raw-bit scope, update the expanded TЗ acceptance criteria
   to match it.
2. If adopting expanded scope, update RTL and manifest:
   - `WORD_BITS`
   - `CLK_DIV`
   - `sample_word_o`
   - `sample_valid_o`
   - `sample_ready_i`
   - valid/ready stream interface
3. Add a behavioral Verilator testbench for reset, grouping, and backpressure.
4. Add report warning: audio quality is not verified because PCM is not
   generated.

Exit criteria:

- `AC-PDM-001..007` are either implemented or explicitly narrowed in docs.

## Priority Recommendation

Highest-value next work:

1. Complete manifest/RTL inspector gaps: ports, clock/reset port binding,
   expanded schema compatibility.
2. Upgrade backend/report contracts so LiteX/Yosys can be added cleanly.
3. Add LiteX skeleton generation with golden tests.
4. Add missing documentation set and ADRs.
5. Decide the `af-pdm-rx` scope before changing RTL behavior.

Do not start vendor production flows until the report/artifact/backend contract
is stable.
