# ADR 0002: Manifest-First Design

`af` treats `af-core.toml` as the source of packaging truth and uses shallow RTL
inspection for consistency checks. It does not implement a full SystemVerilog
front end in MVP.

Deep parsing can be added later through Verible, tree-sitter SystemVerilog,
sv-parser, or Surelog/UHDM adapters.
