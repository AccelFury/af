# Core Author Guide

Create a core directory with:

```text
my-core/
  af-core.toml
  rtl/
  tb/
```

Start with manifest-first declarations. The MVP inspector checks that declared source files exist and that `rtl.top` appears in source text as a module or VHDL entity.

Use `af core new <dir> --name <name> --language verilog` when the core must
start as a Verilog-2001 portable IP block. Use SystemVerilog when the target
integration and lint/sim backends already allow it.

Use `af core new <dir> --name <name> --language verilog --profile reset-sync`
for an atomic reset synchronizer starter. That profile emits a portable
`clk`/`arst`/`rst` core, `STAGES` parameter metadata, and an active-low wrapper
without bus, FIFO, RAM, DSP, PLL, or board-pin logic.

Recommended workflow:

```bash
af manifest validate my-core/af-core.toml
af core check my-core
af core lint my-core --backend verilator
af wrapper generate my-core --target fusesoc
af core report my-core
```

For `verilog` and `verilog-2001` manifests, `af core check` also applies a
portable base-core policy: `default_nettype none` is required, while
SystemVerilog constructs, common vendor macro markers, hidden PLL markers, and
AXI-only markers are rejected in base RTL.

Keep `known_limitations` explicit. Reports include these limitations so downstream users do not confuse MVP checks with signoff.

For buyer-grade cores, keep `af-core.toml`, OpenSpec contracts, integration
docs, release claims, and CI reports in sync. Missing backend support or weak
diagnostics should become an `af` code fix or an explicit `TODO.md` entry.
