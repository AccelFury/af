# Technical Design

`af-cli` owns command parsing and output formatting. It delegates parsing,
inspection, backend planning, wrapper generation, board validation, security,
CI generation, and reports to workspace crates.

Core contracts:

- `af-manifest`: typed TOML parser with compatibility normalization for flat
  and expanded `af-core.toml` shapes.
- `af-rtl-inspector`: manifest-first RTL checks using shallow top module header
  extraction, not a full HDL parser.
- `af-backend`: backend IDs, capabilities, build plans, command specs, tool
  info, diagnostics, and reports.
- `af-security`: path normalization, no-shell command execution, command
  policy fields, toolchain manifest parsing, and redaction helpers.
- `af-wrapper-gen`: target-specific generated wrapper artifacts.

Backend adapters must prepare `program + args[]` commands only. They must not
receive or construct arbitrary shell strings.
