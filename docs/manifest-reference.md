# Manifest Reference

`af-core.toml` v0.1/v0.2/v0.3/v0.4 describes one IP core.

If `[rtl].language` is omitted, the parser defaults to `verilog-2001`.

Required root fields:

- `af_version = "0.1"`, `af_version = "0.2"`, `af_version = "0.3"`, or
  `af_version = "0.4"`
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
- `[[ports]]`: `name`, `direction`, optional `width`, `clock`, `reset`,
  `description`
- `[[clocks]]`: `name`, optional `frequency_hz`
- `[[resets]]`: `name`, optional `active`, `asynchronous`
- `[[interfaces]]`: `name`, `kind`, optional `clock`, `reset`
- `[[testbenches]]`: `name`, `top`, `sources`
- `[[vectors]]` in v0.2: `name`, `format`, `path`

For honest atomic cores that do not have a clock or reset, v0.4 accepts explicit
RTL modes instead of fake ports:

```toml
[rtl]
top = "mux_tree"
language = "verilog-2001"
clocking = "none"
reset = "none"
```

When `rtl.clocking = "none"` is present, `[[clocks]]` may be omitted. When
`rtl.reset = "none"` is present, `[[resets]]` may be omitted. Port and width
checks remain active, and any declared clock/reset reference must still resolve.

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
path = "../af-stream-skid-buffer" # optional same-workspace dependency path

[dependencies.cores.parameter_overrides]
DATA_BITS = "PAYLOAD_BITS"

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

Optional v0.3 semantic contracts:

```toml
[contracts.fifo]
kind = "single_clock"                     # single_clock | dual_clock
interface = "wr_rd_control"               # wr_rd_control | ready_valid
read_mode = "first_word_fall_through"     # first_word_fall_through | registered_read
full_write_policy = "accept_when_full_with_read" # or reject_when_full; allow_when_same_cycle_read is accepted as a legacy alias
clear_behavior = "sync_flush"             # none | sync_flush | async_flush
overflow_policy = "backpressure_no_drop"  # backpressure_no_drop | drop_new | drop_old | flag_only

[[contracts.protocols]]
name = "packet_stream"
kind = "stream"
interface = "ready_valid"
clock = "clk"
reset = "rst"
data_width = "DATA_BITS"

[contracts.protocols.semantics]
payload = "packet_word"
backpressure = "ready_valid"

[[contracts.reset_modes]]
name = "async_active_low"
reset = "rst"
active = "low"
asynchronous = true

[contracts.reset_modes.parameter_overrides]
RESET_ACTIVE_LOW = "1"
ASYNC_RESET = "1"
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
display_name = "Example Core"
summary      = "One-line marketplace summary."
license      = "Apache-2.0"
authors      = ["Example"]
repository   = "https://example.invalid/repo"
homepage     = "https://example.invalid/cores/example_core"
description  = "Long-form description, may span several lines."

# Structured maintainers — coexists with `authors`. Useful for marketplace
# listings and audit trails. All fields except `name` are optional.
[[metadata.maintainers]]
name     = "Example Maintainer"
email    = "core-maint@example.invalid"
role     = "rtl-lead"        # free-form (e.g. rtl-lead, releases, security)
homepage = "https://example.invalid/people/maintainer"
```

`summary` is the short, marketplace-card form; `description` stays the
long-form narrative. `homepage` is the public product page (distinct from
`repository`, which is the source URL). `maintainers` is additive and does
not replace `authors`; tools may render either or both.

See [docs/semver-policy.md](semver-policy.md) for how additions and removals
to this block flow into the public contract.

Validation rules:

- all source, include, evidence and artifact manifest paths must be relative and
  must not contain `..`;
- port widths must be positive integers, parameter names, or simple
  parameter/integer expressions such as `"DATA_BITS"` or `"FIFO_ADDR_BITS + 1"`;
- port/interface clock and reset references must be declared;
- `contracts.protocols[].clock` and `reset` must reference declared clocks,
  resets, or bound ports;
- RTL language must be `systemverilog`, `verilog`, `verilog-2001`, or `vhdl`;
- `sources.files` must not be empty.
- v0.3 constructor export requires `[constructor].category` and
  `[constructor].compatibility_profile` when `export = true`;
- v0.3 resource contracts must have positive memory width/depth and DSP count.
- optional dependency `path` entries may use same-workspace relative paths such
  as `../af-sync-fifo`; `af manifest validate` and `af core check` canonicalize
  them, require an `af-core.toml` at the target, and fail closed if the path
  leaves the current workspace root.

`af core check` also compares manifest port widths against the top RTL module
declaration. A parameterized RTL bus such as `[DATA_BITS-1:0]` should be
declared as `width = "DATA_BITS"` rather than `width = 1`.

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
- `AF_VERIFICATION_EVIDENCE_PLANNED` (warning) when a gate is declared without
  `evidence`.

`af registry check` cross-validates these axes against
`registries/cores.registry.json`. Use `af core registry list` to query the
universal-core registry by priority and portability level.

## Declarative evidence (optional, v0.3)

When a `ReusableCoreMaturity` row would otherwise be reconstructed only from
`--build-root/reports/*` (CI, vendor synthesis, board bring-up), the manifest
may declare the evidence directly. This is structured **input**, not a
fabricated assertion: the matching row only flips to `supported` when the
declared `commit_sha` matches HEAD and `conclusion == "success"`; otherwise the
row stays `planned` with a reason.

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

All `report_path` values are validated through the same relative-path rules as
`[sources]` (no `..`, no absolute prefix). `tool` is bounded by a closed
vocabulary. Unrecognised values raise `AF_EVIDENCE_VENDOR_TOOL_INVALID` or
`AF_EVIDENCE_CONCLUSION_INVALID`.

## Standards profile artifacts (optional, v0.4-compatible)

`[standards]` lets a core opt in to a machine-readable evidence profile without
making the whole `af` manifest FPGA-standards-only. The first built-in profile
is `fpga-ip-core-v1`; it backs `af core standards check`, the additive standards
summary in `af core report`, and the generated root `CHECKLIST.md`,
`compliance_matrix.csv`, and `compliance_matrix.json`.

```toml
[standards]
profile = "fpga-ip-core-v1"

[[standards.artifacts]]
kind = "ip-xact"
path = "ipxact/my_core.xml"
standard = "IEEE 1685"
edition = "2022"
category = "now"
required_for = [24]
conclusion = "present"
sha256 = "optional-hex-digest"

[[standards.artifacts]]
kind = "security-threat-model"
path = "security/threat_model.md"
category = "foundation"
required_for = [31]
conclusion = "placeholder"

[[standards.artifacts]]
kind = "spdx-header-audit"
path = "reports/spdx-header-audit.json"
category = "now"
required_for = [21]
conclusion = "passed"
```

Artifact paths use the same relative-path validation as source and evidence
paths. `required_for` accepts checklist item ids `1..32`. Safety/security
artifacts are evidence hooks only; declaring them does not create a
certification claim.

`af core standards check` reports `validation_status` per checklist row plus
`artifact_validations` per discovered artifact. Supported statuses are
`presence`, `schema-valid`, `semantic-valid`, and `not-applicable`; `partial`
means at least one additive artifact kind is still missing. Invalid or partial
rows stay blocked/planned with an explicit limitation. The top-level
`gates.commercial_baseline_ready` gate is `passed` only when all `now` rows have
evidence; it is not a certification claim.

`af core standards scaffold <core_dir>` can create the conventional evidence
tree for existing cores. It never overwrites existing evidence files; the
generated content is a placeholder that must be filled before release claims.
Add `--declare` to append `[standards]` and `[[standards.artifacts]]` entries
for generated or already-present conventional files.
`af core new
--standards-profile fpga-ip-core-v1` uses the same declaration path
at scaffold time, so newly created opt-in cores are immediately checkable by
manifest evidence rather than only by path convention.
`af core standards check --strict` opportunistically runs external validators
for selected artifacts: `xmllint` for IP-XACT and `peakrdl` for SystemRDL. If
those tools are unavailable, the built-in semantic result is kept and the row
records a limitation; if an available validator rejects the artifact, the row
fails closed. `af core standards doctor` exposes the same local tool
availability probe plus install/container hints, while `af core standards
drift`
reports whether the profile snapshot date needs manual refresh for fast-moving
pins such as SPDX and CWE.

`af core regs scaffold --declare` appends a `systemrdl` artifact declaration for
`regs/<core>.rdl`. `af core standards spdx-audit --declare` appends
`spdx-header-audit` evidence for item 21.
`af core standards collect
--build-root <path> --declare` links known CI/package
outputs, such as `reports/standards/core-lint.json`,
`reports/standards/core-sim.json`, `reports/standards/core-formal.json`, and
`hbom/<core>.spdx.json`, without adding duplicate manifest entries.

`af ci init --standards --standards-core-dir <path>` writes an `[standards]`
section to `af-ci.toml` for CI workflow rendering. The generated CI job runs
native lint and SPDX/HBOM packaging, then calls
`af core standards collect
--declare` and `af core standards check --strict`.
The CI preset assumes an `af` binary is available in PATH or an af-enabled
container/profile is used.
