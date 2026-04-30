<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Verilog-2001 Compatibility Lane

This directory contains drop-in Verilog sources for IP users whose vendor flow
does not accept the SystemVerilog core subset in `rtl/core/`.

Rules:

- sources are valid Verilog-2001 and therefore valid Verilog-2005;
- module names intentionally match the SystemVerilog lane;
- compile exactly one language lane per build filelist;
- keep reset, latency, ready/valid and parameter behavior identical to the
  SystemVerilog implementation;
- do not add packages, interfaces, assertions, typed parameters, `logic`,
  `always_ff`, `always_comb`, unpacked array ports, structs, classes or vendor
  primitives here.

Use:

```bash
cargo xtask lint-rtl --language verilog-2001
cargo xtask sim-verilator --language verilog-2001
cargo xtask lint-rtl --language verilog-2005
cargo xtask sim-verilator --language verilog-2005
```
