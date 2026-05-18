# Software Requirements Specification

## Functional Requirements

- FR-001: Read `af-core.toml` and validate schema/kind.
- FR-002: Check source/include/testbench/formal paths.
- FR-003: Check top module presence by manifest-first inspection.
- FR-004: Check ports, clocks, resets, and clock domains for consistency.
- FR-005: Run Verilator lint/smoke when available.
- FR-006: Generate deterministic FuseSoC `.core`.
- FR-007: Generate LiteX wrapper skeleton without modifying handwritten RTL.
- FR-008: Build backend plans with program/args command specs.
- FR-009: Preserve backend command output in reports/log artifacts.
- FR-010: Generate JSON and Markdown reports.
- FR-011: Return stable exit codes.
- FR-012: Run in GitHub Actions without vendor tools.
- FR-013: Support offline policy through `af-toolchain.toml`.
- FR-014: Record external tool versions.

## Non-Functional Requirements

- NFR-001: Linux-first.
- NFR-002: macOS best effort.
- NFR-003: Windows through WSL2 initially.
- NFR-004: Deterministic output for identical inputs/tool versions.
- NFR-005: Machine-readable reports are mandatory.
- NFR-006: Markdown reports are mandatory.
- NFR-007: CLI must not panic on broken fixtures.
- NFR-008: Small-core checks should complete in seconds.
- NFR-009: Backends should be pluggable behind backend contracts.
- NFR-010: Errors must include code, message, hint, and exit code.

## Security Requirements

- SR-001: No shell interpolation by default.
- SR-002: Tool execution is modeled as executable plus argv.
- SR-003: Manifest paths are normalized and path traversal is rejected.
- SR-004: Generated files are written under build/output roots.
- SR-005: User scripts require explicit future opt-in.
- SR-006: No hidden network access.
- SR-007: RTL/testbench/logs/reports are not uploaded by the CLI.
- SR-008: Vendor tool/license state is warning-only.
- SR-009: Tool versions are reported.
- SR-010: Commands log argv/cwd/policy fields.
- SR-011: Reports should redact obvious secret material.
