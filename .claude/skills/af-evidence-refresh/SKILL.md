---
name: af-evidence-refresh
description: Re-run the full open-source evidence cascade for a single core (lint → sim → synth → wrapper → optional CI-record ingest) so that `af core report` evidence rows flip from `planned`/`blocked` to `supported` where the local toolchain allows. Emits an archive directory with all artefacts and a `SHA256SUMS` file. Use when the user says "refresh evidence", "rerun all gates", "rebuild reports", or "evidence rotted after a commit". Do NOT use to fabricate evidence the local environment cannot actually produce.
allowed-tools: Bash, Read, Glob
---

# af-evidence-refresh

Evidence rows in `af core report::ReusableCoreMaturity` flip to `supported` only when concrete artefacts exist under the build root or have been ingested. After RTL or manifest changes, that evidence is stale — old artefacts may still satisfy the matcher (false-supported) or may have been deleted (false-blocked). This skill runs the deterministic cascade that produces fresh artefacts for every gate the local toolchain can prove, then writes a `SHA256SUMS` manifest so external consumers can attest reproducibility.

## When to invoke

User says or implies:

- "refresh evidence for `<core>`"
- "rerun all gates"
- "evidence rotted"
- "rebuild reports"
- "i changed RTL, prove it still works"
- "what's my current maturity verdict"

Do not invoke for fresh scaffolds — they need `af-bootstrap-core`, which already runs `core check`/`architecture check`/`core report` once. This skill is for *iteration on a populated core*.

## Required inputs

1. **`core_dir`** — directory containing `af-core.toml`. Required.
2. **`build_root`** — defaults to `.af-build/` at repo root; override via `--build-root <path>`.
3. **`targets`** — optional subset of evidence rows to refresh. Default: all open-source rows (everything except `vendor_tool_evidence` and `board_hardware_evidence`).
4. **`ci_record`** — optional path to a CI run-record JSON for ingest (closes `docker_ci_cd_evidence`). Without it, that row stays at its current state.
5. **`dry_run`** — boolean; if true, prints the planned cascade and stops.
6. **`baseline_report`** — optional path to a previous `af core report` JSON for diff comparison at the end.

## Evidence row → command map

Source of truth: `crates/af-report/src/lib.rs::reusable_core_maturity` (substring markers) and `crates/af-report/src/lib.rs::docker_ci_cd_evidence_row` (CI logic).

| Row | What populates it | Commands this skill runs |
|---|---|---|
| `manifest_contract` | manifest parses and validates | `af manifest validate <core_dir>/af-core.toml --json` |
| `source_portability` | manifest declares sources + `core check` passes | `af core check <core_dir> --json` |
| `open_source_tool_evidence` | artefact names contain any of: `native`, `verilator`, `yosys`, `sby`, `openfpga-ci`, `evidence`, `core-check`, `core-sim`, `lint_report`, `simulation_report`, `formal_report`, `synthesis_report`, `pnr_report` | `af core lint --backend native`; gated by `af doctor`: `af core lint --backend verilator`, `af core sim --backend verilator`/`icarus`, `af core lint --backend yosys`, `af core formal --backend sby` |
| `wrapper_package_compatibility` | artefact names contain: `fusesoc`, `litex`, `ipxact`, `.core` | `af wrapper generate <core_dir> --target fusesoc`; `--target ipxact`; `--target litex --board <first_board>` (only if `manifest.boards[]` non-empty) |
| `docker_ci_cd_evidence` | `.af-build/reports/evidence/ci_run_report-*.json` with `conclusion == "success"` AND `commit_sha == HEAD` AND `sha256sums` populated | `af evidence ingest --kind ci-run --input <ci_record> --status <conclusion>` if `ci_record` provided; otherwise leaves the row as-is |
| `vendor_tool_evidence` | artefact names contain: `vivado`, `quartus`, `gowin`, `efinity` | **Not refreshed by this skill** — vendor tooling is out of scope. User must run vendor flow manually and `af evidence ingest --kind synthesis-report --tool <v>` separately. |
| `board_hardware_evidence` | manifest's `boards[]` cross-references against `boards.registry.json` non-placeholder rows | **Not refreshed by this skill** — board bring-up evidence requires physical hardware. |
| `release_support_legal_evidence` | `metadata.license` set + LICENSE/COMMERCIAL-LICENSE/NOTICE files exist | Confirmed by `af core check` (already in cascade) |
| `evidence_portability` | aggregation of the others | derived |
| `buyer_grade_readiness` / `enterprise_grade_readiness` | aggregation | derived |

## Procedure

### Step 1 — probe the toolchain

```bash
cargo run --quiet -p af-cli --bin af -- doctor --json
```

Parse `tool_versions[]`. Build a set of available tools: `verilator`, `iverilog`, `vvp`, `yosys`, `sby`, plus any others present. Use this set to conditionally include backend stages below; do not invoke a backend whose tool is absent — `af` will return `AF_BACKEND_UNAVAILABLE` (exit 4), which is *not* a real failure but pollutes the run.

### Step 2 — emit the plan

In dry-run mode, print the plan table from step 3 and exit. Otherwise proceed.

### Step 3 — run the cascade

In this exact order. Stop on first hard failure (exit codes 2/3/5/6/7/8/11/12 — see `docs/cli-reference.md`). Continue past soft failures (exit code 4 — `AF_BACKEND_UNAVAILABLE`).

```bash
# Always
cargo run --quiet -p af-cli --bin af -- manifest validate <core_dir>/af-core.toml --json
cargo run --quiet -p af-cli --bin af -- core check <core_dir> --json --build-root <build_root>
cargo run --quiet -p af-cli --bin af -- core lint <core_dir> --backend native --json --build-root <build_root>

# If verilator detected
cargo run --quiet -p af-cli --bin af -- core lint <core_dir> --backend verilator --json --build-root <build_root>
cargo run --quiet -p af-cli --bin af -- core sim <core_dir> --backend verilator --json --build-root <build_root>

# If iverilog + vvp detected
cargo run --quiet -p af-cli --bin af -- core sim <core_dir> --backend icarus --json --build-root <build_root>

# If yosys detected
cargo run --quiet -p af-cli --bin af -- core lint <core_dir> --backend yosys --json --build-root <build_root>

# If sby detected AND manifest has [formal] enabled = true
cargo run --quiet -p af-cli --bin af -- core formal <core_dir> --backend sby --json --build-root <build_root>

# Always (wrappers)
cargo run --quiet -p af-cli --bin af -- wrapper generate <core_dir> --target fusesoc --build-root <build_root>
cargo run --quiet -p af-cli --bin af -- wrapper generate <core_dir> --target ipxact --build-root <build_root>

# If manifest declares ≥1 board
cargo run --quiet -p af-cli --bin af -- wrapper generate <core_dir> --target litex --board <first_board> --build-root <build_root>

# Optional CI ingest
[ -n "$ci_record" ] && \
  cargo run --quiet -p af-cli --bin af -- evidence ingest \
    --kind ci-run --input <ci_record> --status <conclusion_from_record> \
    --build-root <build_root>
```

For each command, capture: exit code, JSON output, stdout/stderr log paths (the `--build-root` carries them under `<build_root>/logs/`).

### Step 4 — produce the consolidated report

```bash
cargo run --quiet -p af-cli --bin af -- core report <core_dir> --json --build-root <build_root> \
  > <core_dir>/evidence/report.json
```

Read `maturity.verdict` and the `rows[].status` map.

### Step 5 — write `SHA256SUMS`

`af` itself does not generate the bundle; do it via `sha256sum`:

```bash
mkdir -p <core_dir>/evidence
( cd <build_root> && find reports logs -type f \( -name '*.json' -o -name '*.md' -o -name '*.log' -o -name '*.txt' \) -print0 \
  | sort -z | xargs -0 sha256sum ) > <core_dir>/evidence/SHA256SUMS
sha256sum <core_dir>/evidence/report.json >> <core_dir>/evidence/SHA256SUMS
```

The `( cd ... && find )` ensures relative paths inside `SHA256SUMS` are stable across machines (paths start with `reports/...`, not absolute).

### Step 6 — optional diff against baseline

If `--baseline-report <prev.json>` was provided, compute per-row deltas:

```bash
jq -s '
  .[0].maturity.rows as $a
  | .[1].maturity.rows as $b
  | [ range(0; [$a, $b] | map(length) | max) as $i
      | { area: ($b[$i].area // $a[$i].area),
          before: ($a[$i].status // "absent"),
          after: ($b[$i].status // "absent") }
      | select(.before != .after) ]
' <baseline_report> <core_dir>/evidence/report.json
```

(or implement the comparison directly in skill logic; the jq one-liner is the spec.)

## Required output

```
## Evidence refresh — `<core_dir>` @ commit `<short_sha>`

Build root: `<build_root>`
Tools detected: <comma-list>
Tools skipped: <comma-list with reason "absent">

## Cascade

| Step | Command | Status | Artefact |
|---|---|---|---|
| 1 | `af manifest validate ...` | passed | — |
| 2 | `af core check ...` | passed | `<build_root>/reports/core-check.json` |
| 3 | `af core lint --backend native ...` | passed | `<build_root>/reports/core-lint-native.json` |
| 4 | `af core lint --backend verilator ...` | passed/skipped/failed | `...` |
| ... | | | |

(only include steps that actually ran or were intentionally skipped)

## Maturity after refresh

Verdict: `<supported|partial|blocked>`

| Row | Before | After |
|---|---|---|
| `manifest_contract` | <s> | <s> |
| `source_portability` | <s> | <s> |
| `open_source_tool_evidence` | <s> | <s> |
| `wrapper_package_compatibility` | <s> | <s> |
| `docker_ci_cd_evidence` | <s> | <s> |
| `vendor_tool_evidence` | <s> | <s> (skipped — out of scope) |
| `board_hardware_evidence` | <s> | <s> (skipped — out of scope) |
| `release_support_legal_evidence` | <s> | <s> |

(If `--baseline-report` was not provided, show only the "After" column.)

## Archive

`<core_dir>/evidence/`:
- `report.json`           — consolidated `af core report` output
- `SHA256SUMS`            — hashes over `<build_root>/{reports,logs}/`

## Next gates

(list rows still not `supported` with one-line action each — same mapping as `af-verify-tier`)
```

## Test Design Obligation

When this skill modifies `af`, it must add thoughtful tests for the touched
behavior. Cover success, failure, deterministic JSON/error output, and evidence
boundaries where applicable; if no direct test is possible, state the reason
and cite the closest existing coverage.

## Hard rules

- **Never run a backend whose tool is absent.** Detect first via `af doctor`; skip with explicit reason. The `AF_BACKEND_UNAVAILABLE` exit is not an RTL failure, but invoking those backends pollutes the run with noise.
- **Never invoke vendor tooling.** Vivado/Quartus/Gowin/Efinity are out of scope. The user runs those manually and ingests the report via `af evidence ingest`.
- **Never fabricate a CI record.** If `--ci-record` is not provided, `docker_ci_cd_evidence` keeps its current status. Do not invent commit SHA, run URL, or SHA256SUMS for a run that did not happen.
- **`SHA256SUMS` paths are relative to `<build_root>`.** Absolute paths in the hash file make the bundle non-portable across machines.
- **No partial archive.** If any cascade step beyond the first hard-failure point produces files, the archive can still be written, but the output table must mark every skipped/failed step explicitly. Do not silently exclude artefacts.
- **Stop on hard failures.** Exit codes 2/3/5/6/7/8/11/12 in step 3 abort the cascade. Surface the failing command's JSON, hand off to `af-error-explainer`. Do not continue to wrapper/report stages on a failed `core check`.
- **No git mutations.** This skill does not commit, does not stage, does not push. The user owns version control.

## Edge cases

| Situation | Treatment |
|---|---|
| Core has no `tb/` folder | `af core sim` will fail with a clear error; surface it. User must add a testbench before sim evidence exists. |
| Manifest declares `[formal] enabled = false` | Skip the SBY step; it would be a no-op anyway. |
| `manifest.boards[]` is empty | Skip the `litex --board` wrapper step. |
| User passes `--targets open_source_tool_evidence` | Skip the wrapper generation block; skip CI ingest. Run only the lint/sim/formal cascade. |
| `--baseline-report` references a missing file | Proceed without diff; warn in output. |
| `<build_root>` is shared with other cores | Per-core artefacts live under `<build_root>/reports/<core>-*.json`; `SHA256SUMS` only hashes what was produced for this core (filter by mtime > start-of-run). |
| User wants to refresh a SystemVerilog core | Same cascade applies; `core lint --backend native` will reject SV in portable scope. Hand off to `af-debug-portable-violation`. |

## Example session

```
User: refresh evidence for examples/af-reset-sync
```

Skill probes doctor → finds `iverilog`, `yosys` available; `verilator`, `sby` absent.

Plan emitted:
1. `manifest validate` ✓
2. `core check` ✓
3. `core lint --backend native` ✓
4. `core sim --backend icarus` (iverilog present)
5. `core lint --backend yosys`
6. `wrapper generate --target fusesoc`
7. `wrapper generate --target ipxact`
8. `core report` → archive

Output mirrors the canonical template above with `vendor_tool_evidence` and `board_hardware_evidence` marked "skipped — out of scope".
