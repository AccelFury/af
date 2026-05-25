# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

`af` (AccelFury IP Toolchain) is a Rust-first CLI for FPGA/IP projects. It is
manifest-driven (`af-core.toml`), deterministic, and intentionally honest about
what each command proves: it does not perform timing closure, CDC/RDC signoff,
vendor bitstream generation, or board signoff. The only public binary is `af`,
produced by the `af-cli` crate. Pre-alpha; interfaces may change before v1.0.

## Build, Lint, Test

Stable Rust toolchain (see `rust-toolchain.toml`). Run from repo root.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run a single test (workspace-wide selector):

```bash
cargo test --workspace <test_name>
cargo test -p <crate-name> -- <test_name_pattern>
```

The CLI binary is invoked as:

```bash
cargo run -p af-cli --bin af -- <subcommand> [--json] [--build-root <path>]
```

Docker smoke (canonical open-source toolchain check; installs
Verilator/Yosys/FuseSoC/SMT solvers inside the image):

```bash
make smoke           # docker-build + docker-smoke
make docker-shell    # interactive shell inside the runtime
```

Optional Deno checks (only if Deno is installed; tasks defined in `deno.json`):

```bash
deno fmt --check
deno lint
deno test --allow-read
deno task check:boards
deno task check:toolchains
deno task audit:repo          # writes docs/board_matrix.md
```

Repository self-check (runs in-tree examples and any locally configured external
targets):

```bash
cargo run -p af-cli --bin af -- self check --json
cargo run -p af-cli --bin af -- self check --include-optional --json
```

Fuzz targets are kept outside the normal workspace under `fuzz/`. Run them
manually or in a scheduled/nightly profile, not as part of default CI:

```bash
cargo +nightly fuzz run manifest_toml -- -runs=1024
cargo +nightly fuzz run security_paths -- -runs=1024
```

The self-check manifest is `af-selfcheck.toml`. Optional external targets are
resolved via env vars (e.g. `AF_SELF_CHECK_AF_MOD_ADD`,
`AF_SELF_CHECK_AF_RESET_SYNC`) so a public checkout stays reproducible without
private paths.

## Workspace Architecture

The Cargo workspace lives at the repository root (`Cargo.toml`) with ~30 crates
under `crates/`. The dependency direction is one-way: `af-cli` orchestrates;
library crates do not depend on the CLI.

Core layering:

- `af-cli`: argument parsing, JSON/human output, lifecycle logging, command
  orchestration. Single `af` binary.
- `af-manifest`: `af-core.toml` parsing and validation (v0.3 schema).
- `af-core`: combines manifest validation with shallow RTL inspection.
- `af-rtl-inspector`: shallow Verilog-2001 structural checks. Not a full
  SystemVerilog parser by design.
- `af-security`: path normalization and no-shell argv execution. All backend
  commands are constructed as argv arrays and executed through this layer —
  never via shell strings.
- `af-complexity`: classifies projects as `simple-portable`,
  `composite-portable`, `complex-vendor-aware`, `system-platform`,
  `product-stack`.
- `af-architecture`, `af-resource-model`, `af-compatibility`, `af-signoff`,
  `af-constructor-export`: offline analyses that read manifest contracts.
- `af-backend` + `af-backend-*`: trait-based backend adapters (verilator,
  icarus, yosys, sby, nextpnr, fusesoc, litex, flash/openFPGALoader, native,
  vendor). Each backend reports capability/availability separately from
  invocation.
- `af-report`: JSON/Markdown report rendering, reusable-core maturity scoring.
- `af-wrapper-gen`, `af-board-db`, `af-vendor-db`, `af-template`: packaging
  exports (FuseSoC, LiteX skeleton, IP-XACT), board/vendor metadata, scaffold
  templates.
- `af-vectors` + `af-field-ref`: deterministic test-vector generation
  (finite-field reference arithmetic).
- `af-ci`: GitHub Actions HDL CI generation + doctor/validate/improve commands.
- `af-host`: AGPL-licensed imported host-bringup shell, kept as a separate crate
  so licensing stays per-file.

Data flow: `af-cli` → load+validate manifest → domain service
(core/architecture/resource/compatibility/etc.) → backend adapter constructs
argv → `af-security` executes → `af-report` collects
versions/commands/artifacts/warnings/limitations → JSON+Markdown written under
`--build-root`.

## Repository Conventions

- All generated outputs go under `.af-build/` (configurable via `--build-root`
  or `AF_BUILD_ROOT`). Never write build artifacts elsewhere.
- Scratch/private/imported archives belong in ignored paths (`.af-build/`,
  `.af-tools/`, `.codex`, `archive/`); never commit them to public docs or
  reports.
- Public docs must not link to private/local workspace paths.
- RTL outside an explicit `vendor/<vendor>/` layer must stay portable and
  vendor-agnostic. `af architecture check` fails on vendor primitive/PLL/hard-IP
  markers in `rtl/common`.
- Do not claim timing, CDC/RDC, security, vendor, or board signoff unless a
  command report or vendor artifact proves it. `AF_BACKEND_UNAVAILABLE` means
  missing optional tooling, not an RTL failure.
- Generated reusable IP cores from `af core new` use
  `AccelFury Source Available License v1.0` (SPDX
  `LicenseRef-AccelFury-Source-Available-1.0`); `af core check` fails closed on
  placeholder/mismatched legal policy.
- Stable exit codes are documented in `docs/cli-reference.md` (0 success, 2
  validation, 3 RTL/backend, 4 backend-unavailable, 6 sim, 7 lint, 8 formal, 9
  build, 10 flash, 11 security, 12 artifact missing). Don't reuse codes for
  unrelated failures.
- Every CLI error carries `code`, `message`, `hint`, `exit_code`. Prefer adding
  a new structured code over a free-form message.

## Automation/LLM Guidance

- Always prefer `--json` for diagnostics and reports — that is the contract
  surface.
- Read `docs/cli-reference.md` before inventing command names, flags, schemas,
  or exit codes. Do not invent subcommands.
- The `af-toolchain.toml` policy block (`offline = true`,
  `allow_network = false`) blocks networked installs unless the run passes
  `--allow-network`. Respect it.
- `af tooling check/plan/ensure` is the remediation surface for missing host
  tools. Default install mode is `docker`; system package installation requires
  explicit `--install-mode system --allow-system --allow-network --yes`.

## Hard rules (always active)

These guard the public contract of `af` and the manifesto. Apply
unconditionally.

1. **Use `--json` for any `af` invocation from automation.** Human stderr is
   advisory only; never parse it.
2. **Build artifacts live under `--build-root` (default `.af-build/`).** Never
   write outside it. Treat `.af-build/`, `.af-tools/`, `.codex`, and `archive/`
   as ignored scratch.
3. **Error codes follow `AF_<DOMAIN>_<CONDITION>`** and every error must carry
   `code` + `message` + `hint` + `exit_code`. Reuse existing codes when the
   condition matches; never invent ad-hoc strings.
4. **Manifesto-axes are coupled.** Changes to
   `portability_level`/`priority`/`maturity`/`verification_required` in an
   `af-core.toml` must stay consistent with the corresponding entry in
   `registries/cores.registry.json`. Divergence is fail-closed.
5. **Adding a CLI subcommand or flag updates four places at once:** the clap
   enum in `crates/af-cli/src/main.rs`, the `*_command_name` lifecycle function,
   `docs/cli-reference.md`, and an integration test in
   `crates/af-cli/tests/cli.rs`. Skipping any of the four is a contract
   regression.
6. **Bump `AfReport.schema_version` / `report_version` only with a
   `CHANGELOG.md` entry** describing the shape change and migration. Adding new
   optional fields is non-breaking; renaming or removing existing fields is
   breaking.
7. **Vendor tools are detect-only.** `gw_sh`, `vivado`, `quartus`, etc., are not
   invoked unless the user explicitly passes
   `--allow-network --allow-system --yes`. Never copy vendor netlists (`.dcp`,
   `.xci`, `.edn`, encrypted envelopes) into the repo;
   `AF_PORTABLE_ENCRYPTED_NETLIST` enforces this for portable cores.
8. **LLM output is never evidence.** Maturity rows are computed only from
   artifacts, manifest fields, tool versions, and committed reports. A subagent
   or skill may _describe_ evidence; it must not _fabricate_ it.
9. **Every behavior change needs test design.** Add or update focused tests for
   changed CLI/JSON/schema/registry/backend/report behavior. If no direct test
   is possible, state why in the final response and name the closest existing
   coverage.

## For LLM / AI agents driving `af`

Read [docs/agent-workflow.md](docs/agent-workflow.md) before integrating `af`
into an automated pipeline. The short version:

1. Invoke every command with `--json`. Never parse human stderr.
2. Set `AF_AGENT_NAME` to a stable identifier (e.g. `claude-code/sonnet-4.6`,
   `ci-triager-v1`); it appears in the agent context block of every issue you
   file.
3. On failure: pipe the JSON payload through `af-error-explainer` (subagent) to
   understand it before deciding to file an issue.
4. When you do file: use
   `af agent issue --kind <k> --from-error <file> --title "..." --output <body.md>`
   to render a body, then `af agent gh-url ...` or `af agent gh-cli ...`.
   **`af agent` never submits** — submission is your explicit action.
5. Run `gh search issues` (or invoke the `af-issue-author` subagent which does
   it for you) before creating a new issue. Dedupe is your responsibility.
6. Do not claim signoff `af` did not return. The `evidence-first` rule applies
   to agents the same way it applies to humans.
7. When you modify the tool, design tests before or alongside the change. Cover
   success, failure, deterministic output, and evidence boundaries for the
   touched surface.

## Test Design Obligation

All agents and skills that modify this repository must preserve the test
contract in [docs/testing-strategy.md](docs/testing-strategy.md). A valid
implementation includes thoughtful tests: unit, functional, integration,
property, or fuzz tests for the changed behavior. If no direct test is possible,
state the reason and cite the closest existing coverage.

## Subagents and skills shipped with this repo

Located under `.claude/`:

Subagents (`.claude/agents/`):

- `af-error-explainer.md` — translate a structured `af` failure into a 1–3 step
  fix. Invoke proactively on non-zero exit.
- `af-issue-author.md` — prepare GitHub issue bodies/URLs/CLI commands for human
  submission after an `af` failure or agent-raised request.
- `af-registry-curator.md` — cross-validation between
  `registries/cores.registry.json`, in-tree manifests, `ip_categories.json`, and
  `boards.registry.json`. Read-only audit.
- `af-report-reader.md` — turn an `af core report --json` payload into a
  tier-agnostic action plan grouped by effort.

Skills (`.claude/skills/`):

- `af-bootstrap-core/` — `af core new` + `check` + `architecture check` +
  `report`, with tier-readiness preview.
- `af-migrate-manifest/` — v0.1/v0.2 → v0.3 plus manifesto-axes seeded from the
  cores registry.
- `af-debug-portable-violation/` — locate offending line(s) for any
  `AF_PORTABLE_*` issue and propose the minimal layer-boundary refactor.
- `af-evidence-refresh/` — re-run the open-source evidence cascade
  (checks/lint/sim/formal/wrappers/optional CI-record ingest) and archive
  `SHA256SUMS`.
- `af-verify-tier/` — `af core verify --tier <t>` plus per-missing-row closing
  commands.
- `af-cli-contract-guard/` — pre-commit contract guard for
  CLI/JSON/error/manifest/registry surfaces.
- `af-add-subcommand/` — four-place CLI scaffolder (clap enum + lifecycle name +
  docs + tests). Compiles a TODO-stub handler.
- `af-add-evidence-row/` — four-place evidence-row scaffolder (row builder +
  tier mapping + docs + tests).
- `scripts/check-af-skills.sh` — read-only freshness guard for active
  Claude/Codex skill, agent, and rule surfaces.
- `af-error-explainer/test.sh` — regression test for the explainer subagent
  (enumerates AF_* codes; asserts origin + hint + no-hardcoding).

Project Codex skills (`skills/`):

- `skills/af-heal/`, `skills/af-update/`, `skills/af-upgrade/` — golden standard
  for installable Codex `af-*` skills.
- `scripts/install-af-codex-skills.sh` — installs project `skills/af-*` into
  `${CODEX_HOME:-$HOME/.codex}/skills`.

## Documentation Map

Primary references (read these before adding CLI surface, manifest fields, or
reports):

- `docs/cli-reference.md` — every subcommand, flag, exit code.
- `docs/manifest-reference.md` — `af-core.toml` schema (v0.3).
- `docs/architecture.md` — crate boundaries and data flow.
- `docs/af-ci.md`, `docs/af-ci-config.md`, `docs/af-ci-targets.md`,
  `docs/af-ci-security.md` — generated HDL CI behavior.
- `docs/testing-strategy.md` — required test layers; default CI must pass
  without Verilator/FuseSoC/LiteX/Yosys/SBY/vendor tools installed.
- `docs/known-limitations.md` — what `af` explicitly does not prove.
- `docs/security-model.md`, `docs/licensing.md`, `docs/vendor-tooling.md`,
  `docs/docker-runtime.md`.

`TODO.md` tracks active lifecycle-tool gaps (e.g. CI evidence gating);
`CHANGELOG.md` records user-visible changes per release.
