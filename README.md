# AccelFury `af`

`af` is a Rust CLI for building reusable FPGA/IP cores as verifiable
engineering artifacts: manifest, portable RTL policy checks, simulation and
formal hooks, packaging metadata, CI, standards evidence, reports, and clear
readiness limits.

The project is in **alpha**. The supported command surface is documented in
[docs/cli-reference.md](docs/cli-reference.md); JSON contracts, manifest
schema details, and some reports may still change before v1.0.

## Why It Exists

FPGA work often gets stuck between a useful RTL block and something another
engineer can safely reuse. `af` closes that gap by making evidence explicit:
what the core is, which portability rules it follows, which tools were run,
which wrappers and package formats exist, and which claims are still not
proven.

Use `af` when you want to:

- scaffold a portable or vendor-aware FPGA/IP core with a consistent layout;
- validate `af-core.toml` manifests and RTL portability rules;
- generate FuseSoC, LiteX, IP-XACT, HBOM, and CI artifacts;
- collect lint/sim/formal/package evidence into machine-readable reports;
- prepare cores for FPGA-community reuse without claiming fake signoff;
- give coding agents a deterministic backend instead of hand-written guesses.

`af` does not generate production HDL "by magic". It helps make real FPGA/IP
work repeatable, inspectable, and easier to publish.

## Who It Helps

- IP-core authors who want reusable, documented, portable cores.
- FPGA teams that need CI, reports, wrappers, and readiness gates.
- Open-source maintainers preparing cores for external users.
- Commercial evaluators who need provenance, standards evidence, and honest
  limitations before integration.
- AI-assisted workflows that need stable CLI contracts and JSON output.

## Quick Start

Prerequisites: Linux, `git`, `make`, `python3`, and a stable Rust toolchain.
Optional HDL tools include Verilator, Icarus Verilog, Yosys, SymbiYosys,
Verible, `xmllint`, and PeakRDL. Docker can be used when you do not want to
install HDL tools on the host.

```bash
# Check the local environment
cargo run -p af-cli --bin af -- doctor --json

# Create a portable FPGA/IP core with standards placeholders
cargo run -p af-cli --bin af -- core new ./work/af-demo \
  --name af-demo \
  --class simple-portable \
  --standards-profile fpga-ip-core-v1

# Validate the core and produce a report
cargo run -p af-cli --bin af -- core check ./work/af-demo --json
cargo run -p af-cli --bin af -- core report ./work/af-demo --json

# Check standards evidence and optional external tooling
cargo run -p af-cli --bin af -- core standards doctor --json
cargo run -p af-cli --bin af -- core standards check ./work/af-demo --strict --json
```

Generate wrappers and CI:

```bash
cargo run -p af-cli --bin af -- wrapper generate ./work/af-demo --target fusesoc
cargo run -p af-cli --bin af -- wrapper generate ./work/af-demo --target ipxact
cargo run -p af-cli --bin af -- ci init --standards --standards-core-dir ./work/af-demo
```

Try a complete reference layout:

```bash
cargo run -p af-cli --bin af -- core standards check examples/standards-ready-core --strict --json
```

## What `af` Does

- Creates portable, composite, and vendor-aware core scaffolds.
- Validates `af-core.toml` manifests and portable RTL policy.
- Generates wrapper/package metadata for FuseSoC, LiteX, and IP-XACT.
- Collects lint, simulation, formal, package, and standards evidence.
- Reports reusable-core maturity without promoting unsupported claims.
- Maintains board, toolchain, core, schema, and standards surfaces.

## Common Workflows

- **Start a core:** `af core new` creates the manifest, RTL, legal files,
  docs, and optional standards evidence placeholders.
- **Check portability:** `af core check` rejects common reuse blockers such as
  hidden PLLs, vendor primitives, encrypted netlists, implicit resets, and
  SystemVerilog-only constructs in generic portable cores.
- **Run evidence:** `af core lint`, `af core sim`, package commands, and CI
  jobs produce reports that can be collected as standards artifacts.
- **Package integration:** `af wrapper generate` emits FuseSoC, LiteX, and
  IP-XACT outputs from the manifest-first model.
- **Assess readiness:** `af core report`, `af core verify`, and standards
  checks show what is supported, missing, planned, or out of scope.

See [docs/core-author-guide.md](docs/core-author-guide.md) and
[docs/manifest-reference.md](docs/manifest-reference.md) for the full core
authoring flow.

## Standards Profile

The optional `fpga-ip-core-v1` profile helps IP authors prepare evidence that
FPGA users and commercial integrators expect:

- IP-XACT component metadata;
- SystemRDL register description skeletons;
- SPDX headers and HBOM output;
- lint/sim/formal/package report collection;
- standards drift checks;
- safety/security scaffolds that do not claim certification.

Start with:

```bash
cargo run -p af-cli --bin af -- core standards doctor --json
cargo run -p af-cli --bin af -- core standards scaffold ./work/af-demo --declare
cargo run -p af-cli --bin af -- core standards collect ./work/af-demo --declare --json
cargo run -p af-cli --bin af -- core standards check ./work/af-demo --strict --json
```

The canonical checklist and traceability matrix are in
[CHECKLIST.md](CHECKLIST.md) and [compliance_matrix.csv](compliance_matrix.csv).

## Using Skills

This repository includes workflow skills for contributors and coding agents.
They are plain Markdown instructions, so they can be followed manually or used
by compatible assistants.

- `.claude/skills/**` contains contributor workflows for this repository:
  bootstrapping cores, debugging portability violations, refreshing evidence,
  verifying tiers, and guarding CLI contracts.
- `skills/af-*` contains installable Codex skills. Refresh a local Codex
  mirror with:

```bash
bash scripts/install-af-codex-skills.sh
```

Validate skill consistency after edits:

```bash
bash scripts/check-af-skills.sh
```

## LLM and Automation Guidance

For automation and agents, prefer `--json` on `af` commands and read
[docs/agent-workflow.md](docs/agent-workflow.md) before inventing command
names, flags, JSON shapes, or issue templates.

- Keep generated outputs under `.af-build/` or another explicit build root.
- Keep local scratch notes and private working artifacts in ignored workspace
  paths such as `.af-build/`, `.af-tools/`, `archive/`, and agent caches.
- Do not claim timing, CDC/RDC, security, vendor, or board signoff unless a
  command report or vendor artifact proves it.

## What `af` Does Not Prove

`af` fails closed on unsupported claims. It does not prove timing closure,
CDC/RDC signoff, board hardware readiness, security certification, or vendor
implementation signoff unless specific evidence has been captured and linked.

For current claim boundaries, see
[docs/production-readiness.md](docs/production-readiness.md) and
[docs/known-limitations.md](docs/known-limitations.md).

## Repository Map

- `crates/` - Rust crates and the CLI implementation.
- `examples/` - reusable core examples and standards-ready reference core.
- `docs/` - CLI, manifest, architecture, CI, licensing, and release guides.
- `registries/` - board, core, family, and toolchain registries.
- `schemas/` - JSON schemas for public machine-readable contracts.
- `skills/` and `.claude/skills/` - reusable contributor/agent workflows.

## Development Checks

Before opening a PR or publishing changes:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.claude/skills/af-cli-contract-guard/check.sh
bash scripts/check-af-skills.sh
```

Keep generated outputs under `.af-build/` or another explicit build root.
Do not commit `target/`, `.af-build/`, per-core `artifacts/`, local agent
state, IDE state, secrets, or raw scratch logs.

## Contributing

Open an issue if something is broken, unclear, or missing. Issue templates live
under [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE). For CLI, JSON,
manifest, registry, schema, or error-code changes, read
[CONTRIBUTING.md](CONTRIBUTING.md) and run the contract guard before a PR.

## Licensing

The repository preserves file-level licensing. Rust crates use the workspace
package metadata, imported tooling and RTL keep their original SPDX terms, and
full license texts live in [LICENSES/](LICENSES/).

Reusable IP cores created by `af core new` use the AccelFury source-available
core license boundary with `LICENSE`, `COMMERCIAL-LICENSE.md`, and `NOTICE`
files. For closed-source or commercial use beyond community terms, see
[COMMERCIAL.md](COMMERCIAL.md).
