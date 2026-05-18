# AccelFury `af`

> **Status: pre-alpha, actively developed.**
> CLI surface, JSON shapes, manifest schema, and reports may still change
> before v1.0. If something is missing, broken, or unclear, please **open
> an issue** — that is the single best way to influence what lands next.
> See [Contributing](#contributing) for the issue templates and the
> recommended pre-PR checks.

`af` is a Rust-first CLI for FPGA / IP development. It turns each `af_*` core
(or any third-party core wrapped in an `af-core.toml` manifest) into a
verifiable product: portable Verilog-2001, a manifest, tests, optional
formal properties, synthesis, machine-readable reports, declared
limitations, documentation, board wrappers, and an honest readiness
status.

`af` does **not** "generate HDL by magic". It is the deterministic backend
that powers `af_*` core lifecycles: manifest validation, classification,
simulation, synthesis hooks, packaging, CI scaffolding, board/vendor
adapters, and a documented integration surface for `fpga.chat` / online
constructors (see [docs/fpga-chat-backend.md](docs/fpga-chat-backend.md)).

## Quick Start

Prereqs: a Linux host with `git`, `make`, `python3`, and a stable Rust
toolchain (`rustup` policy is your call; `af` does not bootstrap Rust for
you). A C/C++ toolchain is only required for the optional HDL backends
(Verilator, Yosys, Icarus, SymbiYosys, etc.). Optional: Docker, when you
don't want to install HDL tools on the host.

```bash
# Sanity-check the toolchain
cargo run -p af-cli --bin af -- doctor --json

# Inventory the universal-core registry
cargo run -p af-cli --bin af -- core registry list --json

# Scaffold a new portable IP core
cargo run -p af-cli --bin af -- core new ./work/af-demo --name af-demo --class simple-portable

# Validate it
cargo run -p af-cli --bin af -- core check ./work/af-demo --json
cargo run -p af-cli --bin af -- core report ./work/af-demo --json
```

For the full, current command list:
[docs/cli-reference.md](docs/cli-reference.md).

If you'd rather not install host HDL tools, the Docker runtime ships a
preconfigured open-source HDL stack:

```bash
make smoke
```

Optional OSS HDL tooling on Debian/Ubuntu hosts:

```bash
scripts/pre-install.sh --yes
export PATH="$PWD/.af-tools/python/bin:$PATH"
```

## What `af` Does

- Creates portable, composite, and complex vendor-aware core scaffolds.
- Parses and validates `af-core.toml` manifests (v0.1 / v0.2 / v0.3).
- Classifies a project as `simple-portable`, `composite-portable`,
  `complex-vendor-aware`, `system-platform`, or `product-stack`, and
  surfaces manifesto portability levels `U0..U4`.
- Runs portable-RTL policy checks (Verilog-2001 only; rejects vendor
  primitives, encrypted netlists, hidden PLL/clock managers, AXI-only
  ports, implicit resets, SystemVerilog-only constructs).
- Plans resource intent offline from manifest contracts.
- Scaffolds vendor backend directories without generating fake working
  RTL.
- Exports FuseSoC, LiteX skeletons, IP-XACT, and constructor metadata.
- Reports reusable-core maturity with fail-closed evidence rows for
  manifest contract, source portability, open-source / vendor tool
  evidence, wrapper packaging, current-tree CI evidence, board hardware
  evidence, and release / legal readiness.
- Verifies tier eligibility (`community`, `verified-package`,
  `enterprise`) via `af core verify --tier <t>`.
- Generates GitHub Actions HDL CI with simulation, Yosys JSON synthesis,
  artifact policy, and CI doctor/validate reports.
- Maintains board, toolchain, and universal-core registries.

## What `af` Does Not Prove

- No timing closure or vendor implementation signoff.
- No CDC / RDC signoff (declared formal gates remain `planned` until
  evidence is committed).
- No bitstream production flow by default.
- No security or side-channel certification.
- No claim that generic RTL maps to optimal vendor RAM / FIFO / DSP
  resources.

When a target requires vendor RAM, FIFO, DSP, clocking, constraints, or
board integration, use the complexity-aware templates and backend
contracts instead of treating the design as a small portable core.

## Common Workflows

Scaffold and verify a portable core:

```bash
cargo run -p af-cli --bin af -- core new ./work/af-add --name af-add --class simple-portable
cargo run -p af-cli --bin af -- core check ./work/af-add --json
cargo run -p af-cli --bin af -- core report ./work/af-add --json
```

Scaffold a complex vendor-aware core (memory banking, vendor DSP):

```bash
cargo run -p af-cli --bin af -- core new ./work/af-ntt --name af-ntt --class complex-vendor-aware
cargo run -p af-cli --bin af -- architecture check ./work/af-ntt --json
cargo run -p af-cli --bin af -- resource plan ./work/af-ntt --vendor xilinx --family ultrascale-plus --json
cargo run -p af-cli --bin af -- backend scaffold ./work/af-ntt --vendor xilinx --family ultrascale-plus
```

Verify tier eligibility:

```bash
cargo run -p af-cli --bin af -- core verify ./work/af-add --tier community --json
cargo run -p af-cli --bin af -- core verify ./work/af-add --tier verified-package --json
```

Generate packaging metadata and CI:

```bash
cargo run -p af-cli --bin af -- wrapper generate ./work/af-add --target fusesoc
cargo run -p af-cli --bin af -- wrapper generate ./work/af-add --target litex --board tang-nano-20k
cargo run -p af-cli --bin af -- ci init --project af-add --hdl verilog-2001 --rtl rtl --top af_add --provider github
```

Run the in-tree regression set:

```bash
cargo run -p af-cli --bin af -- self check --json
cargo run -p af-cli --bin af -- self check --include-optional --json
cargo test --workspace
```

The `af-selfcheck.toml` manifest tracks required public examples
(`examples/af-pdm-rx`, `examples/af-mod-add`, `examples/af-reset-sync`,
`examples/simple-counter`) and optional external local cores resolved
via env vars such as `AF_SELF_CHECK_AF_MOD_ADD`.

## LLM and Automation Guidance

`af` is designed to be driven by scripts and coding agents. To avoid
hallucinated flows:

- Prefer `--json` for diagnostics and reports.
- Read [docs/cli-reference.md](docs/cli-reference.md) before inventing
  command names, flags, JSON shapes, or exit codes.
- Keep generated outputs under `.af-build/` or another explicit build
  root.
- Keep local scratch notes and private working artifacts in ignored workspace paths (`.af-build/`, `.af-tools/`, `archive/`, agent caches). They must not appear in `git ls-files`.
- Do not link public documentation to private workspace paths.
- Do not claim timing, CDC / RDC, security, vendor, or board signoff
  unless a command report or vendor artifact proves it.
- If a command returns `AF_BACKEND_UNAVAILABLE`, treat that as missing
  optional tooling, not as an RTL failure.
- Run `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` before proposing public changes.

The contributor skills under `.claude/skills/` and the subagents under
`.claude/agents/` codify these guardrails as reusable workflows — see
the [Contributor skills and subagents](#contributor-skills-and-subagents-claude)
section below.

## Documentation

Reference (read these before opening a PR that touches the CLI surface,
manifest schema, or reports):

- [CLI reference](docs/cli-reference.md) — every subcommand, flag, exit
  code, and the JSON contract.
- [Manifest reference](docs/manifest-reference.md) — `af-core.toml`
  v0.1 / v0.2 / v0.3 fields.
- [Architecture](docs/architecture.md) — crate boundaries, data flow,
  and the U0..U4 ↔ `ProjectClass` taxonomy.
- [FPGA.chat backend roles](docs/fpga-chat-backend.md) — how
  Fit Doctor / Core Doctor / Constructor / Report Engine / Registry
  Sync map to existing `af` commands.

Guides:

- [Core author guide](docs/core-author-guide.md), including the
  buyer-ready checklist.
- [Backend author guide](docs/backend-author-guide.md).
- [Board author guide](docs/board-author-guide.md).
- [CI guide](docs/af-ci.md) and [CI config](docs/af-ci-config.md),
  [CI targets](docs/af-ci-targets.md), [CI security](docs/af-ci-security.md).
- [Testing strategy](docs/testing-strategy.md).
- [Vendor tooling](docs/vendor-tooling.md).
- [Docker runtime](docs/docker-runtime.md).
- [Security model](docs/security-model.md).
- [Licensing](docs/licensing.md) — community, verified-package, and
  enterprise commercial tiers.
- [Known limitations](docs/known-limitations.md).
- [Release process](docs/release-process.md).
- [Board matrix](docs/board_matrix.md) (auto-generated).
- [Dev roadmap](docs/dev-roadmap.md).

## Contributing

This project is moving and welcomes outside help. **The fastest way to
shape it is to open an issue.**

### Open an issue

Use the templates under `.github/ISSUE_TEMPLATE/`:

- **Bug** — something does the wrong thing or returns the wrong error
  code.
- **New IP request** — you want an `af_*` core that is not yet in
  `registries/cores.registry.json`.
- **New board request** — you want a board added to the registry and
  matrix.
- **Board bring-up** — physical-board integration / pinout problem.
- **Feature request / question** — anything else, including "I am not
  sure how to do X with `af`".

If you have a structured `af` failure (a JSON payload from a `--json`
invocation), paste it verbatim. The
[`af-error-explainer`](.claude/agents/af-error-explainer.md) subagent
(used internally) can translate it into a 1–3 step fix.

**If you are an LLM / AI agent**, use `af agent --help` instead of
authoring issues by hand. The CLI renders a pre-filled issue body with
a deterministic `## Agent context` block (af version, commit SHA,
environment hash, repo) and prints a ready-to-paste GitHub URL or
`gh issue create` command line. `af agent` is offline-only — it never
POSTs and never invokes `gh`. The full workflow is in
[docs/agent-workflow.md](docs/agent-workflow.md).

### Before opening a PR

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p af-cli --bin af -- registry check --json
cargo run -p af-cli --bin af -- self check --json
```

For changes that touch CLI / JSON / manifest / registry surfaces, read
[CONTRIBUTING.md](CONTRIBUTING.md) and the contract rules at the bottom
of [CLAUDE.md](CLAUDE.md).

### Best practices

- Prefer `--json` for any `af` invocation in scripts and CI.
- Keep generated outputs under `.af-build/` (or any explicit
  `--build-root`).
- Use the existing
  [contributor skills](#contributor-skills-and-subagents-claude) before
  writing scaffolding by hand.
- Do not commit local IDE / agent state — see `.gitignore`.

### Contributor skills and subagents (`.claude/`)

`af` ships a small set of operational skills and subagents under
`.claude/` that any compatible coding assistant (Claude Code, Codex,
Cursor with skill mapping, etc.) can use. They are normal Markdown
files; you can also follow them by hand.

Subagents:

- `agents/af-error-explainer.md` — translate a structured `af` failure
  into a fix plan.
- `agents/af-registry-curator.md` — read-only audit of
  `registries/cores.registry.json` against examples, categories, and
  boards.
- `agents/af-report-reader.md` — turn `af core report --json` into a
  tier-agnostic action plan.

Skills:

- `skills/af-bootstrap-core/` — scaffold a new core end-to-end.
- `skills/af-migrate-manifest/` — bring a legacy `af-core.toml` to v0.3.
- `skills/af-debug-portable-violation/` — locate and refactor any
  `AF_PORTABLE_*` issue.
- `skills/af-evidence-refresh/` — re-run the open-source evidence
  cascade (lint, sim, synth, wrappers) and produce a `SHA256SUMS`
  bundle.
- `skills/af-verify-tier/` — `af core verify --tier <t>` plus per-row
  closing commands.
- `skills/af-cli-contract-guard/` — pre-commit guard for CLI / JSON /
  error / manifest / registry surfaces.
- `skills/af-add-subcommand/` — four-place CLI scaffolder (clap enum +
  lifecycle name + docs + tests).
- `skills/af-add-evidence-row/` — four-place evidence-row scaffolder.

There is also a regression test at
`.claude/skills/af-error-explainer/test.sh` that enumerates every
`AF_*` error code under `crates/` and asserts it has a real origin and
a real hint string.

Local Claude Code permissions
(`.claude/settings.local.json`) are gitignored on purpose; the shared
skills above are not.

## Licensing

The repository preserves file-level licensing. Rust crates use the
workspace package metadata, imported tooling and RTL keep their original
SPDX terms, and full license texts live in [LICENSES/](LICENSES/).

Reusable IP cores created by `af core new` use the
`AccelFury Source Available License v1.0` with `LICENSE`,
`COMMERCIAL-LICENSE.md`, and `NOTICE` files. `af core check` fails
closed on placeholder or mismatched legal policy. Three commercial
tiers (`community`, `verified-package`, `enterprise`) are defined in
[docs/licensing.md](docs/licensing.md) and verified by
`af core verify --tier <t>`.

For closed-source / commercial use beyond community terms, see
[COMMERCIAL.md](COMMERCIAL.md).
