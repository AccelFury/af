# Toolchain Notes

## Gowin EDA (`toolchains/gowin_eda`)

- primary flow for all Gowin targets
- CST input, project file generation and report collection are intentionally
  placeholders
- board-level `gowin/build.tcl` scripts own device, top-level naming, and part
  settings
- source language support is generic; both `.v` and `.sv` RTL inputs are
  expected and verified at board scope

## Intel Quartus (`toolchains/intel_quartus`)

- primary flow for Cyclone/MAX10 targets
- QSF constraints first, SDC for timing
- board-level `quartus/build.tcl` keeps pin maps, PLL, and families explicit

## Xilinx Vivado (`toolchains/xilinx_vivado`)

- primary flow for Artix-7 and Zynq-7000 targets
- XDC-first constraints, deterministic top and board part selection
- board-level `vivado/build.tcl` plus constraints define timing strategy

## Yosys + nextpnr iCE40 (`toolchains/lattice_oss` + `lattice_oss_yosys_nextpnr_ice40`)

- open-source flow for iCE40
- PCF constraints and nextpnr command-line scripts
- reproducible environment for CI

## Yosys + nextpnr ECP5 (`toolchains/lattice_oss` + `lattice_oss_yosys_nextpnr_ecp5`)

- LPF constraints and nextpnr ECP5 backend
- board-level LPF should include pin-safe default constraints and optional I/O
  bank notes

## Lattice Radiant (`toolchains/lattice_radiant`)

- vendor fallback for iCE40 / ECP5 where required
- keep core unchanged, use board wrappers for bank and IO standard mapping

## Efinity (`toolchains/efinix_efinity`)

- primary flow for Trion / Titanium targets
- LPF constraints and any vendor constraint timing overrides

## Microchip Libero (`toolchains/microchip_libero`)

- primary flow for PolarFire / IGLOO2 targets
- PDC + optional SDC flows
- hard reset trees and power planning remain board responsibilities

## Verilator (`cargo xtask sim-verilator`)

- RTL simulation in CI uses `verilator --binary`
- waveform capture and PASS/FATAL semantics in SV testbench
- deterministic vectors from Rust generator
- filelist supports both `rtl/*.sv` and `rtl/*.v` sources; strict language flags
  are avoided in favor of extension-based parsing
- `cargo xtask sim-verilator --language systemverilog` uses
  `tb/verilator/filelist.f`
- `cargo xtask sim-verilator --language verilog-2001` uses
  `tb/verilator/filelist_verilog2001.f`
- `cargo xtask sim-verilator --language verilog-2005` uses
  `tb/verilator/filelist_verilog2005.f`
- The runner passes `-CFLAGS -std=c++20` to keep modern Verilator timing support
  compatible with GCC/Clang defaults while still using no user C++ testbench.
- The runner also passes `-MAKEFLAGS VM_PARALLEL_BUILDS=0` to avoid
  environment-specific precompiled-header failures in some packaged Verilator
  builds; this favors determinism over compile speed for template CI.
