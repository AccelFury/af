# FPGA Family Notes

## Gowin families

- **gowin_gw1n / gowin_gw1ns**
  - compact logic and modest BRAM/DSP mix
  - CST constraints, careful vendor PLL placement
  - read/write RAM behavior varies with inferred style

- **gowin_gw2a / gowin_gw5a / gowin_gw5ast**
  - higher block and DSP capacity for stream, NTT, hash, and hash-accumulator
    blocks
  - usually supports larger clock trees; still keep core clock/reset generic
  - prefer generated clocks in board wrappers and `o_ready`/`i_ready` handshake
    in core

**Portability pitfall:** do not assume vendor-specific RAM wrapper availability
in core.

## Lattice iCE40 notes

- **lattice_ice40** (iCEBreaker, TinyFPGA, iCEStick)
  - constraint format: PCF
  - open-source flow through Yosys + nextpnr-ice40
  - strict pinbank review required before attaching peripherals
  - carry-chain behavior differs by package; validate timing closure on
    timing-critical arithmetic

## Lattice ECP5 notes

- **lattice_ecp5** (ULX3S, OrangeCrab, ColorLight)
  - constraint format: LPF
  - open-source flow through Yosys + nextpnr-ecp5
  - support for larger BRAM/DSP blocks, but RAM mode remains family-dependent

## Intel Cyclone IV / MAX10 / Cyclone V SoC notes

- constraint format: QSF/SDC
- Quartus supports broad inference but RAM read/write behavior and timing
  exceptions should be qualified per board
- on SoC variants keep processor subsystem and DDR clocks isolated from pure IP
  wrappers

## Xilinx 7-series / Zynq-7000 notes

- constraint format: XDC
- block RAM/DSP inference is strong but reset fanout, CDC and multicycle paths
  must be reviewed
- clock-domain crossing policy should keep generated clocks in board-level logic

## Efinix Trion / Titanium notes

- constraint format: LPF + optional SDC
- low-cost inference for block RAM; check read-during-write and latency mode
  explicitly
- external memory options often require explicit board-level power sequencing

## Microchip IGLOO2 / PolarFire notes

- constraint format: PDC (+ optional SDC)
- deterministic reset trees are especially important for ASIC-like migration
- verify PLL/clock manager settings in board wrapper and avoid hard reset fanout
  in core

## Generic portability pitfalls

- avoid vendor primitives and attributes in core
- avoid fixed clock domains and hidden resets in reusable modules
- avoid assumptions on inferred memory timing mode
- avoid tool-generated init-only behavior for synthesizable state
- keep all board-specific pin/voltage/voltage-domain assumptions in board
  wrappers
- keep source files plain Verilog-2001/Verilog-2005 (`.v`) or SystemVerilog
  (`.sv`) and avoid unsupported language extensions in `rtl/common` and core
