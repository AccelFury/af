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

Recommended workflow:

```bash
af manifest validate my-core/af-core.toml
af core check my-core
af core lint my-core --backend verilator
af wrapper generate my-core --target fusesoc
af core report my-core
```

Keep `known_limitations` explicit. Reports include these limitations so downstream users do not confuse MVP checks with signoff.

For buyer-grade cores, keep `af-core.toml`, OpenSpec contracts, integration
docs, release claims, and CI reports in sync. Missing backend support or weak
diagnostics should become an `af` code fix or an explicit `TODO.md` entry.
