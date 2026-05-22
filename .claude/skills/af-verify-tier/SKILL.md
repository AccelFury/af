---
name: af-verify-tier
description: Run `af core verify --tier <tier>` against a core and translate every unmet evidence row into the exact `af` command (or external action) that would close it. Use when the user says "is X buyer-grade", "can we claim verified-package", "tier check for <core>", "what's blocking enterprise tier", or "promote <core> to <tier>". Do NOT use to bypass evidence; this skill never marks a row supported on its own.
allowed-tools: Bash, Read, Glob
---

# af-verify-tier

This skill closes the commercial-tier story end-to-end. It runs the deterministic tier-check in `af` and converts each missing evidence row into an actionable command that produces real evidence. It is the **only** sanctioned path for asking "is this core in tier X". It is not the path for *making* a core be in tier X — that requires actual work.

## When to invoke

User says any of:

- "verify <core> for community/verified-package/enterprise tier"
- "is <core> buyer-grade"
- "tier check"
- "what's blocking <core> from verified-package"
- "promote <core> from community to verified-package"

## Required inputs

1. **`core_dir`** — path to a core directory containing `af-core.toml`. If omitted and run inside an `af` repo, list candidates from `examples/` and `af-selfcheck.toml::targets` and ask once.
2. **`tier`** — `community`, `verified-package`, or `enterprise`. If omitted, default to `verified-package` (the most informative middle tier) and tell the user.

Other inputs (build root, JSON) are fixed by this skill.

## Tier → required rows (cheat-sheet — re-verify via source if doubt)

Source of truth: `crates/af-cli/src/main.rs::tier_required_rows` and
`docs/licensing.md::Commercial tiers`.

| Tier | Required rows (must all be `supported`) |
|---|---|
| `community` | `manifest_contract`, `source_portability` |
| `verified-package` | above + `open_source_tool_evidence`, `wrapper_package_compatibility`, `docker_ci_cd_evidence` |
| `enterprise` | above + `vendor_tool_evidence`, `board_hardware_evidence`, `release_support_legal_evidence` |

## Procedure

### Step 1 — run the verify command

```bash
cargo run --quiet -p af-cli --bin af -- core verify <CORE_DIR> --tier <TIER> --json
```

Two outcomes:

- **Exit 0** (`"status": "passed"`): all required rows are `supported`.
  Skip to the "passed" output template.
- **Exit 2** (`AF_TIER_REQUIREMENTS_UNMET`): print details and proceed.

If `AF_TIER_UNKNOWN`: the user misspelled the tier. Ask once.

### Step 2 — parse the `missing` array

The structured payload is in `details.tier_verification.missing` (or
`tier_verification.missing` when exit was 0). Each entry has:

```json
{
  "area": "<row name>",
  "status": "supported|blocked|planned|not-applicable",
  "evidence": [...],
  "limitations": [...]
}
```

`limitations[]` already tells the user *what is missing*. Your job is to
translate each into *what command produces the evidence*.

### Step 3 — map missing rows to closing actions

This mapping is canonical. Quote the user the exact command, with paths
substituted, when the row is missing.

| Missing row | Closing command(s) | Notes |
|---|---|---|
| `manifest_contract` | `af manifest validate <CORE_DIR>/af-core.toml --json` | If this is missing, the core has no `af-core.toml`. Re-run `af-bootstrap-core` first. |
| `source_portability` | `af core check <CORE_DIR> --json` then fix any `AF_PORTABLE_*` issue per `af-error-explainer` | `[sources]` must be non-empty; portable-Verilog policy must pass. |
| `open_source_tool_evidence` | `af core lint <CORE_DIR> --backend native --json` (always available); add `--backend verilator`, `--backend yosys` if installed; `af core sim <CORE_DIR> --backend icarus --json` if `iverilog` is in `PATH` | The row populates from artifact names containing `native`, `verilator`, `yosys`, `sby`, `core-check`, `simulation_report`, `synthesis_report`, etc. |
| `wrapper_package_compatibility` | `af wrapper generate <CORE_DIR> --target fusesoc`; optionally `--target litex --board <board>`; optionally `--target ipxact` | At least one wrapper artifact required. |
| `docker_ci_cd_evidence` | (a) `af ci init --project <name> --hdl verilog-2001 --rtl rtl --top <top> --provider github`; (b) push and wait for a run; (c) `af ci doctor --repo .` + archive `SHA256SUMS` + run-record JSON under `<CORE_DIR>/evidence/ci/`; (d) re-run verify | A workflow file alone is `planned`, not `supported`. The gate requires a current-tree run record AND a SHA256SUMS bundle. See `TODO.md` (closed) AF.TODO.CI-CURRENT-TREE-EVIDENCE-GATE. |
| `vendor_tool_evidence` | `af evidence ingest --kind synthesis-report --input <vivado_or_quartus_or_gowin_report> --tool <vendor> --status passed` | `af` cannot run vendor tools itself. The user must provide a vendor report; `af` only ingests it. |
| `board_hardware_evidence` | Current row cannot be made `supported` by a single CLI command. `af build <core> --board <id> --backend nextpnr --json` and `af evidence ingest --kind hardware-measurement --input <bringup_log.json> --status passed` can archive evidence, but `board_hardware_evidence` currently remains `planned`/`not-applicable` until the maturity model consumes hardware-measurement evidence. | Do not claim enterprise tier if this row is not `supported`. `af evidence ingest --kind board-bringup` is not a valid command. |
| `release_support_legal_evidence` | Ensure `[metadata].license` is set in `af-core.toml`, then run `af core check <CORE_DIR> --json`. | The row itself is driven by `metadata.license`; `core check` covers broader legal-file policy for generated cores. |

### Step 4 — verify-after-fix loop boundary

Do **not** re-run any of those commands automatically. Tell the user
which to run, then stop. If they ask you to run them, do so one at a time
and re-invoke this skill at the end.

### Step 5 — output

**Pass case** (`status: passed`):

```
## ✅ `<vlnv>` satisfies tier `<tier>`

All <N> required rows are `supported`. Maturity verdict: `<verdict>`.

If you intend to claim this externally, archive the JSON report alongside
the release artefacts:

```bash
cargo run --quiet -p af-cli --bin af -- core report <CORE_DIR> --json \
  > <CORE_DIR>/evidence/<tier>-tier.json
```
```

**Fail case** (`AF_TIER_REQUIREMENTS_UNMET`):

```
## ❌ `<vlnv>` does not satisfy tier `<tier>`

Missing rows (<N>):

| Row | Status | Why blocked |
|---|---|---|
| `<area>` | `<status>` | <first limitation, truncated to ~80 chars> |

## To close each row

### `<area>`

```bash
<exact command(s) per the mapping above, with paths substituted>
```

<one-line note about what evidence the row will then find — e.g. "this writes `core-check.json` under `--build-root`, which populates `open_source_tool_evidence`">

(repeat for each missing row)

## After closing all rows

```bash
cargo run --quiet -p af-cli --bin af -- core verify <CORE_DIR> --tier <tier> --json
```
```

## Test Design Obligation

When this skill modifies `af`, it must add thoughtful tests for the touched
behavior. Cover success, failure, deterministic JSON/error output, and evidence
boundaries where applicable; if no direct test is possible, state the reason
and cite the closest existing coverage.

## Hard rules

- **No row promotion shortcuts.** The skill must never edit `af-core.toml`, `cores.registry.json`, or any evidence file to "make" a row supported. Tier promotion happens only by producing real evidence.
- **Quote tiers verbatim.** Do not invent intermediate tiers; the three names are fixed.
- **Do not run vendor tools.** If `vendor_tool_evidence` is missing, the right action is "user produces a vendor report and `af evidence ingest`s it". `af` does not orchestrate Vivado/Quartus/Gowin EDA.
- **Do not claim a row is supported because a file exists.** The maturity computation reads the manifest plus report/evidence records under `--build-root/reports/` (including `--build-root/reports/evidence/`). If the user has artifacts elsewhere, suggest regenerating under the same `--build-root` or using `af evidence ingest` where the current maturity model consumes that evidence.
- **Stay terse.** This skill produces a table and a few code blocks. No prose summaries, no encouragement.
- **Forbid claim language.** Do not produce strings like "ready for production", "drop-in replacement", "buyer-grade certified". Only structural status (`supported` / `planned` / `blocked` / `not-applicable`).

## Example session

User: `verify examples/af-reset-sync for verified-package`

You run `af core verify examples/af-reset-sync --tier verified-package --json`.
Output (paraphrased):

```
❌ accelfury:utility:af_reset_sync:0.1.0 does not satisfy tier `verified-package`

Missing rows (2):

| Row | Status | Why blocked |
|---|---|---|
| `wrapper_package_compatibility` | blocked | Run wrapper generation for FuseSoC, LiteX, or IP-XACT to populate this row. |
| `docker_ci_cd_evidence`         | blocked | No CI configuration or CI run evidence was discovered. |

## To close each row

### wrapper_package_compatibility

```bash
cargo run --quiet -p af-cli --bin af -- wrapper generate examples/af-reset-sync --target fusesoc
```

### docker_ci_cd_evidence

```bash
cargo run --quiet -p af-cli --bin af -- ci init --project af-reset-sync --hdl verilog-2001 --rtl rtl --top af_reset_sync --provider github
# then push, wait for one green run, archive the run JSON + SHA256SUMS under examples/af-reset-sync/evidence/ci/
```

## After closing all rows

```bash
cargo run --quiet -p af-cli --bin af -- core verify examples/af-reset-sync --tier verified-package --json
```
```
