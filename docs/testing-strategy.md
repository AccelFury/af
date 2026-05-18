# Testing Strategy

Required test layers:

- manifest unit tests for flat and expanded schemas;
- security unit tests for path traversal and command specs;
- RTL inspector tests for source/top/port/clock/reset failures;
- golden tests for FuseSoC and LiteX generated files;
- report snapshot tests for JSON and Markdown;
- CLI integration tests for happy path and broken fixtures;
- repository self-checks from `af-selfcheck.toml` for required examples and
  locally available optional standalone core projects;
- fake backend tests to prove no shell strings are needed.

Default CI must pass without Verilator, FuseSoC, LiteX, Yosys, SBY, or vendor
tools installed.

The Docker CI job is the canonical open-source toolchain check. It installs
Verilator, xmllint, FuseSoC, Edalize, Yosys and formal SMT solvers, then runs
`scripts/docker-smoke.sh` to exercise simulation, packaging, LiteX skeleton
generation, Yosys checks, solver visibility, manifest migration and report
generation. The LiteX Python package is optional in the default image because
MVP LiteX support does not execute a LiteX SoC build.
