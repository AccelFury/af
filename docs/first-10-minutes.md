# First 10 Minutes

This walkthrough creates a portable core, checks it, generates integration
metadata, enables the standards flow, and produces a report.

## 1. Install or run locally

```bash
cargo install --path crates/af-cli
af doctor --json
```

From a checkout, replace `af` with:

```bash
cargo run -p af-cli --bin af --
```

## 2. Create a core

```bash
af core new ./work/af-demo \
  --name af-demo \
  --class simple-portable \
  --language verilog-2001 \
  --standards-profile fpga-ip-core-v1 \
  --json
```

## 3. Check and report

```bash
af manifest validate ./work/af-demo/af-core.toml --json
af core check ./work/af-demo --json
af core report ./work/af-demo --json
```

## 4. Generate wrappers

```bash
af wrapper generate ./work/af-demo --target fusesoc --json
af wrapper generate ./work/af-demo --target ipxact --json
```

## 5. Enable standards flow

```bash
af core standards doctor --json
af core regs scaffold ./work/af-demo --declare --json
af core standards collect ./work/af-demo --declare --json
af core standards check ./work/af-demo --strict --json
```

## 6. Add CI

```bash
af ci init \
  --project af-demo \
  --hdl verilog \
  --rtl rtl \
  --standards \
  --standards-core-dir ./work/af-demo
```

The result is not a timing, CDC/RDC, vendor, board, safety, or security
signoff. It is a reproducible starting point with explicit evidence and
limitations.
