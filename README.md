# AccelFury `af`

`af` is a Rust CLI for turning FPGA/IP-core work into reusable engineering
artifacts: manifests, portable RTL checks, wrappers, CI, standards evidence,
machine-readable reports, and explicit readiness limits.

The project is **alpha**. The stable surface for users and automation is
documented in [docs/cli-reference.md](docs/cli-reference.md); JSON contracts and
some report details may still change before v1.0.

## Why Use It

FPGA projects often stop at "the RTL works on my machine". `af` helps close the
gap between a useful block and something another engineer can evaluate, reuse,
or publish:

- scaffold portable or vendor-aware FPGA/IP cores with a consistent layout;
- validate `af-core.toml` manifests and common portability mistakes;
- generate FuseSoC, LiteX, IP-XACT, HBOM, and CI surfaces;
- collect lint, simulation, formal, package, and standards evidence;
- keep unsupported claims visible instead of implying fake signoff;
- give coding agents a deterministic CLI and `--json` contracts.

`af` is useful for IP-core authors, FPGA teams, open-source maintainers,
commercial evaluators, and AI-assisted workflows that need reproducible project
state rather than hand-written guesses.

## Install

From a checkout:

```bash
cargo install --path crates/af-cli
af doctor --json
```

Without installing:

```bash
cargo run -p af-cli --bin af -- doctor --json
```

Release binaries are published with `SHA256SUMS`; Docker releases use immutable
GHCR digests recorded by the release gate. See
[docs/reproducible-builds.md](docs/reproducible-builds.md).

## Quick Start

Prerequisites: Linux, `git`, `make`, `python3`, and a stable Rust toolchain.
Optional HDL tools include Verilator, Icarus Verilog, Yosys, SymbiYosys,
Verible, `xmllint`, and PeakRDL. Docker can carry the heavier tool profile.

```bash
# 1. Check the host/tool profile.
af doctor --json

# 2. Create a portable core with standards placeholders.
af core new ./work/af-demo \
  --name af-demo \
  --class simple-portable \
  --standards-profile fpga-ip-core-v1 \
  --json

# 3. Validate the manifest and RTL portability policy.
af core check ./work/af-demo --json
af core report ./work/af-demo --json

# 4. Generate integration metadata.
af wrapper generate ./work/af-demo --target fusesoc --json
af wrapper generate ./work/af-demo --target ipxact --json

# 5. Enable standards evidence and CI collection.
af core standards doctor --json
af core standards check ./work/af-demo --strict --json
af ci init --standards --standards-core-dir ./work/af-demo --project af-demo --hdl verilog --rtl rtl --json
```

For a guided walkthrough, see
[docs/first-10-minutes.md](docs/first-10-minutes.md). For ready examples, start
with [examples/README.md](examples/README.md).

## What `af` Does

- **Start a core:** `af core new` creates manifest, RTL, legal files, docs, and
  optional standards placeholders.
- **Check reuse blockers:** `af core check` catches hidden PLLs, vendor
  primitives, encrypted netlists, implicit resets, and unsupported generic-core
  constructs.
- **Package integration:** `af wrapper generate` emits FuseSoC, LiteX, and
  IP-XACT outputs from the manifest-first model.
- **Collect evidence:** lint, sim, formal, package, CI, and standards commands
  produce reports that downstream tools can read.
- **Assess readiness:** `af core report`, `af core verify`, `af release check`,
  and standards checks show what is supported, missing, planned, or out of
  scope.

Full references live in [docs/core-author-guide.md](docs/core-author-guide.md),
[docs/manifest-reference.md](docs/manifest-reference.md), and
[docs/production-readiness.md](docs/production-readiness.md).

## Common Workflows

- **Start:** create a core from a portable, composite, or vendor-aware template.
- **Check:** validate manifest, legal boundary, RTL portability, and standards
  placeholders.
- **Package:** emit integration wrappers and metadata from the manifest.
- **Report:** collect evidence into JSON and Markdown outputs for review.
- **Release:** use `af release check --json` to keep publication claims gated by
  explicit evidence.

## Standards Profile

The optional `fpga-ip-core-v1` profile helps FPGA/IP authors prepare evidence
that users and commercial integrators expect:

- IP-XACT component metadata;
- SystemRDL register-description skeletons;
- SPDX headers and HBOM output;
- lint/sim/formal/package report collection;
- standards drift checks;
- safety/security scaffolds without certification claims.

Start from:

```bash
af core standards doctor --json
af core standards scaffold ./work/af-demo --declare --json
af core standards collect ./work/af-demo --declare --json
af core standards check ./work/af-demo --strict --json
```

The checklist and traceability matrix are in [CHECKLIST.md](CHECKLIST.md) and
[compliance_matrix.csv](compliance_matrix.csv).

## Skills And Agents

This repository includes plain Markdown workflow skills for contributors and
compatible coding agents.

- `.claude/skills/**` contains repo contributor workflows such as CLI contract
  guarding, evidence refresh, portability debugging, and release checks.
- `skills/af-*` contains installable Codex skills. Refresh a local Codex mirror:

```bash
bash scripts/install-af-codex-skills.sh
```

Validate skill consistency after edits:

```bash
bash scripts/check-af-skills.sh
```

## LLM and Automation Guidance

Automation should prefer `--json` and follow
[docs/agent-workflow.md](docs/agent-workflow.md) before inventing command names,
flags, JSON shapes, or issue templates.

## Publication Boundaries

Keep generated outputs under `.af-build/` or another explicit build root. Do not
commit `target/`, `.af-build/`, per-core `artifacts/`, local agent state, IDE
state, secrets, or raw scratch logs; keep these in an ignored workspace.

## What `af` Does Not Prove

`af` fails closed on unsupported claims. It does not prove timing closure,
CDC/RDC signoff, board hardware readiness, security certification, or vendor
implementation signoff unless specific evidence has been captured and linked.

## Repository Map

- `crates/` - Rust crates and CLI implementation.
- `examples/` - reusable core examples and standards-ready references.
- `docs/` - CLI, manifest, architecture, CI, licensing, and release guides.
- `registries/` - board, core, family, and toolchain registries.
- `schemas/` - JSON schemas for public machine-readable contracts.
- `skills/` and `.claude/skills/` - contributor and agent workflows.

## Development Checks

Before opening a PR or publishing changes:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.claude/skills/af-cli-contract-guard/check.sh
bash scripts/check-af-skills.sh
```

## Contributing And Licensing

Use the issue templates under [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE).
For CLI, JSON, manifest, registry, schema, or error-code changes, read
[CONTRIBUTING.md](CONTRIBUTING.md) and run the contract guard.

The repository preserves file-level licensing. Rust crates use workspace package
metadata, imported tooling and RTL keep their original SPDX terms, and full
license texts live in [LICENSES/](LICENSES/). Cores generated by `af core new`
use the AccelFury source-available core license boundary with `LICENSE`,
`COMMERCIAL-LICENSE.md`, and `NOTICE`; commercial terms are summarized in
[COMMERCIAL.md](COMMERCIAL.md).
