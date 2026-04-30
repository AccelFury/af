# Testing Strategy

Required test layers:

- manifest unit tests for flat and expanded schemas;
- security unit tests for path traversal and command specs;
- RTL inspector tests for source/top/port/clock/reset failures;
- golden tests for FuseSoC and LiteX generated files;
- report snapshot tests for JSON and Markdown;
- CLI integration tests for happy path and broken fixtures;
- fake backend tests to prove no shell strings are needed.

Default CI must pass without Verilator, FuseSoC, LiteX, Yosys, SBY, or vendor
tools installed.

The Docker CI job is the canonical open-source toolchain check. It installs
Verilator, FuseSoC, LiteX and Yosys, then runs `scripts/docker-smoke.sh` to
exercise simulation, packaging, LiteX wrapper generation, Yosys checks,
manifest migration and report generation.
