# Architecture

The implementation is split by portability boundary:

- `rtl/core`: arithmetic core and parameterized top
- `rtl/common`: reusable interfaces and utility blocks
- `rtl/vendor`: vendor-specific adapters only
- `boards/*`: board-dependent wrappers and constraints
- `tb/*`: SystemVerilog testbench and source lists
- `crates/*`: Rust tooling and reference model
- `tools/ts/*`: Deno checks and repository quality gates

The top-level module in core is protocol-centric and does not include any hard
IP. All board-specific pinout, LED behavior, and synthesis script assumptions
are kept out of `rtl/core`.
