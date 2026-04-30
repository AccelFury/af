<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Verilator Simulation Artifacts

This directory contains simulation entry points used by the Rust runner.

## Layout

- `filelist.f` — SystemVerilog core file list for `verilator --binary`.
- `filelist_verilog2001.f` — Verilog-2001 core file list with the shared SV
  testbench.
- `filelist_verilog2005.f` — Verilog-2005 core file list with the shared SV
  testbench.
- `README.md` — this document.

## Supported Command Path

The simulation command path is driven from Cargo xtask:

1. Generate vectors:
   - `cargo xtask vectors`
2. Run simulation:
   - `cargo xtask sim-verilator`
   - `cargo xtask sim-verilator --language verilog-2001`
   - `cargo xtask sim-verilator --language verilog-2005`

The runner invokes `verilator --binary` with the files from `filelist.f` and
captures:

- execution log,
- waveform (when enabled),
- pass/fail status (via `tb_af_mod_add.sv` `$fatal` on failure).

## Notes

- No Python harnesses are used.
- No C++ testbench is used.
- The testbench is SystemVerilog (`tb/sv/tb_af_mod_add.sv`) and reads vectors
  from `vectors/af_mod_add_random.svh`.
- RTL sources in `tb/verilator/filelist.f` may be `.v` (Verilog-2001/2005) or
  `.sv` (SystemVerilog).
- Select one language lane per run. Do not include the SystemVerilog and Verilog
  `af_mod_add` implementations together because they intentionally share the
  same module names for drop-in replacement.

## Safety and Policy

- This is a template; `status` flags in board/task flows should be
  `not_measured` until hardware validation is complete.
- Generated vectors are deterministic and intended for CI/reproducibility.
