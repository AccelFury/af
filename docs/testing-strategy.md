# Testing Strategy

`af` is a deterministic CLI and report generator for FPGA/IP workflows. Tests
must protect the public contract as much as the Rust implementation: command
names, JSON shapes, error envelopes, exit codes, manifest fields, build-root
layout, evidence semantics, generated files, and "what af does not prove".

## Required layers

Every feature or fix must choose the smallest set of tests that proves the
changed behavior. The required layers are:

- unit tests for pure library behavior: manifest parsing, security path
  normalization, RTL inspection, resource formulas, compatibility, signoff
  planning, report row logic, backend argv construction, and registry parsing;
- functional CLI tests with `assert_cmd` for public subcommands, `--json`
  output, stable error envelopes, exit codes, no ANSI in JSON, and broken
  fixture handling;
- integration tests for end-to-end workflows such as
  `core new -> manifest
  validate -> core check -> wrapper generate -> package -> report -> verify`;
- snapshot/golden tests for generated JSON, Markdown, FuseSoC, LiteX, IP-XACT,
  schemas, and deterministic reports;
- property and fuzz tests for parsers, path normalization, registry loaders, RTL
  text inspection, vector generation, and CI/workflow input surfaces;
- repository self-checks from `af-selfcheck.toml` for required examples and
  locally available optional standalone core projects;
- fake backend tests that prove commands are argv arrays, never shell strings.

## Coverage rules

Changes to public behavior require both a success case and at least one negative
case. A negative case should assert the exact `AF_*` code, hint-bearing error
envelope, and documented exit code. If the behavior writes files, tests must
assert the files stay under `--build-root` or the explicitly requested output
path.

Changes to deterministic output must include byte-stability or topology tests.
JSON tests should verify sorted/stable fields, no timestamps unless explicitly
part of the contract, and forward-slash paths where the CLI promises portable
output. Generated artifacts must carry the AccelFury generated-by marker.

Evidence and maturity tests must be evidence-first. No row may become
`supported` without a concrete artifact path or evidence record, and every
`blocked` row must carry a limitation or reason. Vendor, board, timing, CDC/RDC,
and security signoff claims require artifacts; tests must reject claims based on
LLM text, comments, or workflow presence alone.

## Fuzzing

Property tests that are fast and deterministic belong in normal `cargo test`.
Long-running fuzzing belongs under `fuzz/` and is run manually or by a scheduled
nightly job. The required fuzz targets cover:

- arbitrary TOML bytes through `CoreManifest::from_toml_str`;
- arbitrary path strings through `normalize_relative_path` and `safe_join`;
- arbitrary board registry bytes through the registry loader;
- arbitrary RTL text inside a minimal temp core through `inspect_core`;
- arbitrary CI target/backend strings through `af-ci` generation APIs.

Fuzz targets must not require network access, vendor tools, hardware, or
committed generated corpora.

## CI profiles

Default CI must pass without Verilator, Icarus, FuseSoC, LiteX, Yosys, SBY, or
vendor tools installed. It runs formatting, clippy, `cargo test --workspace`,
Deno validators, repository self-checks, and CLI smoke tests.

The Docker CI job is the canonical open-source toolchain check. It installs
Verilator, xmllint, FuseSoC, Edalize, Yosys and formal SMT solvers, then runs
`scripts/docker-smoke.sh` to exercise simulation, packaging, LiteX skeleton
generation, Yosys checks, solver visibility, manifest migration and report
generation. The LiteX Python package is optional in the default image because
MVP LiteX support does not execute a LiteX SoC build.

`scripts/oss-hdl-smoke.sh` is the local non-Docker OSS backend matrix. It runs
real Verilator, Icarus, Yosys and SBY command paths over the in-tree examples,
including backend-specific testbench selection and explicit skipped lanes. Use
it when those tools are installed locally; default CI may keep this behind an
OSS-toolchain job rather than the host-only Rust job.

Vendor and hardware checks are gated/manual. Public CI may ingest fixture
reports for `synthesis-report`, `pnr-report`, `programming-log`, and
`hardware-measurement`, but it must not launch proprietary tools or claim board
bring-up without explicit external artifacts.

## Agent obligation

AI/LLM agents that modify `af` must add or update thoughtful tests for the
behavior they change. If a change has no direct test, the agent must state the
reason and list the closest existing coverage. This obligation applies to code,
docs that define contract behavior, schemas, registry entries, generated
artifacts, and `.claude/` skills or agents.
