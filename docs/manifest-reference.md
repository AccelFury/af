# Manifest Reference

`af-core.toml` v0.1/v0.2/v0.3 describes one IP core.

If `[rtl].language` is omitted, the parser defaults to `verilog-2001`.

Required root fields:

- `af_version = "0.1"`, `af_version = "0.2"`, or `af_version = "0.3"`
- `name`
- `vendor`
- `library`
- `core`
- `version`

Required tables:

```toml
[rtl]
top = "my_core"
language = "verilog-2001"
default_clock = "clk"
default_reset = "rst_n"

[sources]
files = ["rtl/my_core.v"]
include_dirs = []
```

Supported arrays:

- `[[parameters]]`: `name`, `value`, optional `description`
- `[[ports]]`: `name`, `direction`, optional `width`, `clock`, `reset`, `description`
- `[[clocks]]`: `name`, optional `frequency_hz`
- `[[resets]]`: `name`, optional `active`, `asynchronous`
- `[[interfaces]]`: `name`, `kind`, optional `clock`, `reset`
- `[[testbenches]]`: `name`, `top`, `sources`
- `[[vectors]]` in v0.2: `name`, `format`, `path`

Optional v0.2 fields:

```toml
category = "field_arithmetic"

[rtl.variants]
systemverilog = ["rtl/core.sv"]
verilog_2001 = ["rtl/core.v"]

[tooling]
rust = true
typescript_deno = true
python = false
cocotb = false
fusesoc_required = false
```

Optional v0.3 complexity and architecture fields:

```toml
[complexity]
class = "complex-vendor-aware"
score = 8
decision = "memory banking and vendor DSP backend required"
triggers = ["memory_banking", "vendor_dsp_backend_required"]

[architecture]
style = "portable_contract_with_vendor_backends"
reference_backend = "generic"

[reuse]
prefer_existing_microcores = true

[[dependencies.cores]]
name = "af-stream-skid-buffer"
version = ">=0.2.0"
role = "ready_valid_boundary"

[[resources.memory]]
name = "work_ram"
kind = "ram_1r1w"
width = 64
depth = 4096
latency_cycles = 1
backend_policy = "prefer_vendor"

[[resources.dsp]]
name = "mul_pipeline"
kind = "multiplier"
count = 64
backend_policy = "require_vendor"

[[backend_variants]]
name = "xilinx_ultrascale_plus"
vendor = "xilinx"
families = ["ultrascale-plus"]
status = "planned"

[constructor]
export = true
category = "compute"
compatibility_profile = "af_stream_v1"
```

Supported `complexity.class` values:

- `simple-portable`
- `composite-portable`
- `complex-vendor-aware`
- `system-platform`
- `product-stack`

`backend_policy` must be `portable`, `prefer_vendor`, or `require_vendor`.
`backend_variants.status` must be `supported`, `planned`, or `unsupported`.
Planned or unsupported backend variants require explicit `known_limitations`.

Optional metadata:

```toml
[metadata]
license = "Apache-2.0"
authors = ["Example"]
repository = "https://example.invalid/repo"
description = "Core description"
```

Validation rules:

- all manifest paths must be relative and must not contain `..`;
- port widths must be positive integers;
- port/interface clock and reset references must be declared;
- RTL language must be `systemverilog`, `verilog`, `verilog-2001`, or `vhdl`;
- `sources.files` must not be empty.
- v0.3 constructor export requires `[constructor].category` and
  `[constructor].compatibility_profile` when `export = true`;
- v0.3 resource contracts must have positive memory width/depth and DSP count.

## Manifesto axes (optional, v0.3)

These root-level fields tag a core against the AccelFury manifesto axes. They
are optional and parallel to `complexity.class`; see
[architecture.md — Core Taxonomy](architecture.md#core-taxonomy) for the
mapping.

```toml
portability_level = "U0"   # U0..U4
priority          = "P0"   # P0, P1, P2
maturity          = "preview"  # experimental | preview | beta | stable | deprecated

[[verification_required]]
kind        = "formal-cdc-assumption"
description = "Async assertion + N-cycle sync deassertion."

[[verification_required]]
kind     = "simulation"
evidence = "reports/smoke.log"   # optional, relative to the core directory
```

`maturity` is core-level. It is distinct from per-backend
`backend_variants[].status` and per-board `boards[].status`.

`verification_required[].kind` accepts: `simulation`, `formal-cdc-assumption`,
`formal-occupancy`, `formal-equivalence`, `random-stress`, `board-demo`,
`synthesis-report`. `af architecture check` reads these gates and reports:

- `AF_VERIFICATION_EVIDENCE_MISSING` (issue) when `evidence` points at a path
  that does not exist;
- `AF_VERIFICATION_EVIDENCE_PLANNED` (warning) when a gate is declared
  without `evidence`.

`af registry check` cross-validates these axes against
`registries/cores.registry.json`. Use `af core registry list` to query the
universal-core registry by priority and portability level.

## Declarative evidence (optional, v0.3)

When a `ReusableCoreMaturity` row would otherwise be reconstructed only from
`--build-root/reports/*` (CI, vendor synthesis, board bring-up), the manifest
may declare the evidence directly. This is structured **input**, not a
fabricated assertion: the matching row only flips to `supported` when the
declared `commit_sha` matches HEAD and `conclusion == "success"`; otherwise
the row stays `planned` with a reason.

```toml
[evidence.docker_ci_cd]
run_url    = "https://github.com/owner/repo/actions/runs/12345"
commit_sha = "abc1234deadbeef..."
sha256sums = "abc1234deadbeef0..."
conclusion = "success"   # or "failure" / "cancelled"

[evidence.vendor_tool]
tool        = "vivado"   # vivado | quartus | gowin | efinity | libero | radiant | diamond
report_path = "reports/vivado/synth.json"
conclusion  = "success"

[evidence.board_hardware]
board_id    = "tang-nano-20k"
report_path = "reports/board/tang_smoke.json"
date        = "2026-05-17"
```

All `report_path` values are validated through the same relative-path rules
as `[sources]` (no `..`, no absolute prefix). `tool` is bounded by a closed
vocabulary. Unrecognised values raise `AF_EVIDENCE_VENDOR_TOOL_INVALID` or
`AF_EVIDENCE_CONCLUSION_INVALID`.
