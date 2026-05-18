---
name: af-bootstrap-core
description: Bootstrap a fresh AccelFury (`af`) FPGA/IP core scaffold from a single command. Generates the directory through `af core new`, runs `af core check`, `af architecture check`, and `af core report`, then returns a one-screen status with the manifesto axes (priority/portability_level/maturity) and the next concrete gate to close. Use when the user says "create a new af core", "scaffold an af_* core", or "init af_<name>". Do NOT use when an existing core needs migrating — that is `af-migrate-manifest`.
allowed-tools: Bash, Read, Write, Edit, Glob
---

# af-bootstrap-core

This skill creates a deterministic, verified-from-the-start core scaffold using the four canonical `af` commands. It is the standard entry point for any new `af_*` core. It does **not** write RTL logic; it produces a scaffold that already passes `af core check` and prints exactly what is still missing to reach the next maturity tier.

## When to invoke

User says any of:

- "create a new af core named X"
- "scaffold af_<thing>"
- "start a new portable Verilog core / composite core / complex vendor-aware core"
- "init <slug> as af core"

## Required inputs

Ask the user (in one batch) if not provided:

1. **`name`** — the core slug. Will be sanitised to a Verilog identifier. Example: `af-pulse-sync`.
2. **`class`** — one of `simple-portable` (default), `composite-portable`, `complex-vendor-aware`. Map manifesto vocabulary: utility/stream/audio → `simple-portable`; memory/bus reusable → `composite-portable`; PLL/DSP/RAM-banking accelerator → `complex-vendor-aware`.
3. **Optional axes overrides** — `--portability-level U0..U4`, `--priority P0..P2`, `--maturity experimental|preview|beta|stable|deprecated`. Default behaviour: profile picks reasonable defaults; only override on explicit request.
4. **`core_dir`** — destination path. Default: `examples/<name>` if the user is working inside the `af` repo, else `./<name>`.

If `class=simple-portable` and the slug starts with `af-reset` or matches a reset/CDC primitive, suggest `--profile reset-sync` (canonical N-stage synchronizer template).

## Procedure

Run the four commands in this order. Stop on the first non-zero exit code and surface it via the `af-error-explainer` subagent.

### Step 1 — scaffold

```bash
cargo run --quiet -p af-cli --bin af -- core new <CORE_DIR> \
  --name <NAME> \
  --class <CLASS> \
  [--profile reset-sync] \
  [--portability-level <U?>] \
  [--priority <P?>] \
  [--maturity <experimental|preview|...>] \
  --json
```

Outputs the generated manifest JSON. Confirm:

- `manifest.portability_level` is set.
- `manifest.priority` is set.
- `manifest.maturity` is set.
- `manifest.verification_required[]` is non-empty.
- `development_artifacts` lists `artifacts/openfpga-ci/README.md`.

If the user passed an invalid axis value, `af` returns `AF_CORE_NEW_AXIS_INVALID` (exit code 2). Re-prompt for the correct value; do not retry blindly.

### Step 2 — manifest + RTL structural check

```bash
cargo run --quiet -p af-cli --bin af -- core check <CORE_DIR> --json
```

Required outcome: `"status": "passed"` and `"portable_verilog_policy": "pass"`. If any portable-policy violation appears (`AF_PORTABLE_*`), hand the JSON to `af-error-explainer`. Common cause for a *fresh* scaffold: user passed `--language systemverilog` — the generated template assumes verilog-2001.

### Step 3 — architecture / verification gates

```bash
cargo run --quiet -p af-cli --bin af -- architecture check <CORE_DIR> --json
```

Expected outcome on a fresh scaffold: `"status": "warning"` with one or more
`AF_VERIFICATION_EVIDENCE_PLANNED` warnings (the `[[verification_required]]`
gates declared in step 1 have no evidence file yet). This is the desired
state — it tells the user what gates to close next.

Hard failures here (status `failed`) indicate generator bug; do not paper
over them. Surface to user verbatim.

### Step 4 — initial evidence report

```bash
cargo run --quiet -p af-cli --bin af -- core report <CORE_DIR> --json
```

Save the `report.maturity` block. Extract:

- `verdict` (will be `blocked` for a fresh scaffold — that is correct, not a bug)
- the `rows[].status` per area (`supported` / `planned` / `blocked`)

### Step 5 — buyer-readiness preview

Read the `tier_required_rows` mapping (documented in
`docs/licensing.md::Commercial tiers`):

- `community` requires: `manifest_contract`, `source_portability`.
- `verified-package` adds: `open_source_tool_evidence`, `wrapper_package_compatibility`, `docker_ci_cd_evidence`.
- `enterprise` adds: `vendor_tool_evidence`, `board_hardware_evidence`, `release_support_legal_evidence`.

For each tier, list which required rows are already `supported` and which
are not. The fresh scaffold will normally satisfy `community` immediately
and need several closes for `verified-package`.

## Required output format

Always exactly this shape (no preamble, no closing pleasantries):

```
## Created `<core_dir>`

- VLNV: `accelfury:<library>:<core>:0.1.0`
- Class: `<class>` · portability `U?` · priority `P?` · maturity `<...>`
- Verification gates declared: <kind>, <kind>, ...
- Files: `af-core.toml`, `LICENSE`, `COMMERCIAL-LICENSE.md`, `NOTICE`, `rtl/<module>.v`<, more if any>

## Tier readiness (now)

| Tier | Status | Missing rows |
|---|---|---|
| community | <passes/fails> | <none / list> |
| verified-package | <fails> | <comma-list> |
| enterprise | <fails> | <comma-list> |

## Next concrete gates

1. <one actionable command, e.g. "Write `tb/tb_<module>.v`, then `af core sim <core_dir> --backend verilator`">
2. <"Run `af wrapper generate <core_dir> --target fusesoc` to close `wrapper_package_compatibility`">
3. <"Generate CI evidence: `af ci init ...`, push, archive run record + SHA256SUMS under `<core_dir>/evidence/ci/`">
```

Do not pad. The user wants the scaffold and the punch list.

## Hard rules

- **Do not write RTL behaviour.** The generated `rtl/<module>.v` from `af core new` is intentionally a skeleton. Skill must not "improve" it — that is `af-portable-coach`'s job and only on explicit request.
- **Do not skip steps 2–4** even if step 1 succeeded. The whole point is to prove the scaffold passes the gates we promise.
- **Do not commit anything.** This skill does not run `git`. Leave staging to the user.
- **Do not invent reference_path in `registries/cores.registry.json`.** That is a separate workflow (`af-registry-curator`).
- **Verilog only.** If the user requests `--language systemverilog` for a base core, refuse with the standard `af` policy: SystemVerilog belongs in wrappers, not in generic cores. Quote the policy line from `docs/core-author-guide.md`.
- **No retries on validation failures.** If step 2/3/4 fails after step 1 succeeded, that is a generator bug or a corrupted source tree. Hand off to `af-error-explainer` and stop.

## Common scenarios

| User says | You do |
|---|---|
| "create af-pulse-sync" | class `simple-portable`, suggest `--profile reset-sync` adjacency check; if user confirms it is a pulse sync (different from reset sync), use the default profile |
| "create a Tang Nano demo core" | refuse silently — board demos are not the scope of `af core new`; suggest the user create a portable core first and a separate board wrapper |
| "scaffold af_uart with priority P0 portability U0" | pass `--priority P0 --portability-level U0 --maturity experimental` to step 1; verify the manifest reflects them |
| "create at /tmp/x" | accept the absolute path; do not force `examples/` |
| "language systemverilog" | refuse, cite policy, suggest verilog-2001 with optional SystemVerilog wrapper |
