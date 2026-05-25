# af-reset-sync

Portable N-stage reset synchronizer with configurable polarity. Manifesto axes:
priority `P0`, portability level `U0`, maturity `preview`. Implemented in
Verilog-2001 with no vendor primitives, no PLL, and no clock-management IP.

Behavior:

- Asynchronous assertion: when `src_rst` enters its asserted level, `dst_rst`
  follows within one clock event in `clk`.
- Synchronous deassertion: after `src_rst` is released, `dst_rst` deasserts only
  after `STAGES` rising edges of `clk`, eliminating metastability from the
  source side.

## Parameters

| Name             | Default | Description                                                    |
| ---------------- | ------- | -------------------------------------------------------------- |
| `STAGES`         | `2`     | Number of synchronizer flops (>=2).                            |
| `RESET_POLARITY` | `0`     | `0` = active-low (`src_rst_n`/`dst_rst_n`); `1` = active-high. |

## Ports

| Name      | Direction | Width | Description                                   |
| --------- | --------- | ----- | --------------------------------------------- |
| `clk`     | input     | 1     | Destination clock domain.                     |
| `src_rst` | input     | 1     | Source reset (polarity per `RESET_POLARITY`). |
| `dst_rst` | output    | 1     | Synchronized destination reset.               |

## Verification gates

Declared in `af-core.toml` and tracked by `af registry check`:

- `formal-cdc-assumption` — `formal/af_reset_sync_props.sv` stub; engage with
  SymbiYosys + vendor reset assumptions before promoting to `stable`.
- `simulation` — `tb/tb_af_reset_sync.v` smoke testbench (Icarus/Verilator).

## What this core does NOT do

- It does not generate clocks. Use vendor clocking IP outside of this core.
- It does not enforce vendor-specific reset distribution. Wrappers (Xilinx INIT,
  Intel CLR, Gowin GSR) live in the board/vendor layer.
- It is not a power-on reset generator. Drive `src_rst` from an upstream POR
  module or board-level reset.

## Manifesto cross-reference

| Axis                | Value     | Meaning                                     |
| ------------------- | --------- | ------------------------------------------- |
| `priority`          | `P0`      | First-class universal core.                 |
| `portability_level` | `U0`      | Fully portable RTL; no vendor primitives.   |
| `maturity`          | `preview` | Smoke-tested; formal gate not yet promoted. |
