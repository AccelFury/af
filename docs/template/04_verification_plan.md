# Verification Plan

1. Unit-level mathematics in `af-field-ref` unit tests.
2. Deterministic vector generation:
   - edge cases (`0+0`, `1+1`, `p-1+1`, `p-1+p-1`)
   - random vectors with fixed seed
3. Verilator simulation:
   - SystemVerilog testbench in `tb/sv/tb_af_mod_add.sv`
   - `cargo xtask sim-verilator`
   - log capture in `reports/simulation/`
4. Lint:
   - `cargo xtask lint-rtl` uses Verilator `--lint-only`
5. Metadata and license checks via `deno task audit:repo` and
   `cargo xtask check-spdx`.
