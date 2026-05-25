---
name: af-add-evidence-row
description: Add a new evidence row to `ReusableCoreMaturity` with all four required touch-points wired up — the row-builder in `crates/af-report/src/lib.rs`, the optional `tier_required_rows` mapping in `crates/af-cli/src/main.rs`, the `docs/licensing.md::Commercial tiers` table, and matching tests. Use when the user says "add a new maturity row for X", "track <Y> evidence", or "extend Reusable-core gates". Do NOT use to modify existing rows; those are breaking changes for `af core verify` and require coordinated CHANGELOG.
allowed-tools: Bash, Read, Edit, Write, Grep
---

# af-add-evidence-row

`af core report::ReusableCoreMaturity` carries 11 evidence rows that together
describe what a core has proven. Adding a new row is structural — every consumer
(`af core verify`, `af-report-reader`, `af-verify-tier`, docs/licensing.md)
depends on the row vocabulary and the matching artifact substrings. This skill
makes the addition consistent in one pass.

## When to invoke

User says:

- "add a new maturity row for X"
- "track `<evidence>` as a separate gate"
- "extend reusable-core gates with `<area>`"

Do NOT invoke for:

- modifying an existing row's matcher or limitations (breaking change; route
  through `af-cli-contract-guard`)
- changing `tier_required_rows` mapping for an existing row (breaking — same)
- adding evidence ingestion _commands_ (different scope — that is a separate
  `af evidence ingest --kind <kind>` extension)

## Required inputs

1. **`area`** — snake_case row name. Must NOT collide with the existing 11:
   `manifest_contract`, `source_portability`, `evidence_portability`,
   `wrapper_package_compatibility`, `open_source_tool_evidence`,
   `vendor_tool_evidence`, `docker_ci_cd_evidence`, `board_hardware_evidence`,
   `release_support_legal_evidence`, `buyer_grade_readiness`,
   `enterprise_grade_readiness`.

2. **`evidence_source`** — how the row is populated:
   - `artifact_substrings`: list of substrings the matcher looks for in
     `artifacts[]` (e.g. `["cocotb", "uvm"]`).
   - `manifest_field`: a specific manifest field whose presence/value drives the
     row (e.g. `metadata.repository` for a hypothetical `provenance_evidence`).
   - `custom`: a Rust function call — user must write the helper themselves;
     skill stubs the entry point only.

3. **`limitations`** — one-line text for the `blocked` status
   (`"No cocotb run discovered under .af-build/."`).

4. **`tier_membership`** — optional. If the row is required for one of the
   existing tiers (`verified-package`, `enterprise`), name which. The skill
   refuses to add a row to `community` (community is intentionally minimal: only
   `manifest_contract` + `source_portability`).

## Touch-points

1. **Row builder** — `crates/af-report/src/lib.rs::reusable_core_maturity`, the
   function that pushes `MaturityRow`s onto `rows`.
2. **Tier mapping** — `crates/af-cli/src/main.rs::tier_required_rows`, the match
   arms returning `&[&str]` lists.
3. **Docs** — `docs/licensing.md::Commercial tiers` table.
4. **Tests**:
   - `crates/af-report/src/lib.rs::tests` (unit-level — supported/blocked
     scenarios for the new row).
   - `crates/af-cli/tests/cli.rs` if the row is part of a tier (integration test
     for `af core verify --tier <t>` covering the new row's blocked state on a
     baseline example).

## Procedure

### Step 1 — collision check

```bash
grep -n 'rows.push(row(' crates/af-report/src/lib.rs | head -30
```

Inventory existing `area` strings. If the requested `area` matches any, refuse:

```
AF_EVIDENCE_ROW_TAKEN: area `<area>` already exists at crates/af-report/src/lib.rs:<line>
```

### Step 2 — choose insertion point

Order in `rows.push(...)` calls determines presentation order in
`af core report` output. Keep ordering meaningful:

- input rows first, grouped by closeness to the toolchain (`manifest_contract` →
  `source_portability` → tool/wrapper/CI evidence)
- vendor/board rows after open-source rows
- legal evidence near the end
- aggregation rows (`*_readiness`) last

A reasonable default for a new input row: insert immediately before
`vendor_tool_evidence` (the last open-source-tool block) unless the user
specifies otherwise.

### Step 3 — wire the row builder

Read the current `reusable_core_maturity` function. For each `evidence_source`
mode:

#### `artifact_substrings`

```rust
let <area>_artifacts = matching_artifacts(artifacts, &[<comma-separated literals>]);
rows.push(row(
    "<area>",
    if <area>_artifacts.is_empty() { "blocked" } else { "supported" },
    <area>_artifacts,
    if <area>_artifacts.is_empty() {
        vec!["<limitations text>".to_string()]
    } else {
        Vec::new()
    },
));
```

`matching_artifacts` is already defined in the same file. Substrings are
case-sensitive substring matches against artifact paths.

#### `manifest_field`

```rust
let <area>_evidence = manifest
    .and_then(|m| m.<field_path>.as_ref())
    .map(|v| vec![format!("<field>: {v}")])
    .unwrap_or_default();
rows.push(row(
    "<area>",
    if <area>_evidence.is_empty() { "blocked" } else { "supported" },
    <area>_evidence,
    if <area>_evidence.is_empty() {
        vec!["<limitations text>".to_string()]
    } else {
        Vec::new()
    },
));
```

#### `custom`

Insert a TODO placeholder calling a user-defined helper, with the row defaulting
to `planned`:

```rust
rows.push(row(
    "<area>",
    "planned",
    Vec::new(),
    vec!["<limitations text> (helper unimplemented)".to_string()],
));
// TODO: replace with call to <user_helper_fn>(manifest, artifacts) once defined.
```

### Step 4 — wire tier mapping (only if `tier_membership` is non-empty)

In `crates/af-cli/src/main.rs::tier_required_rows`, add the row name to the
appropriate tier's array. **Important**: `verified-package` includes the
`community` rows; `enterprise` includes both. Maintain the inheritance.

Example: adding `cocotb_evidence` to `verified-package`:

```rust
"verified-package" => Ok(&[
    "manifest_contract",
    "source_portability",
    "open_source_tool_evidence",
    "wrapper_package_compatibility",
    "docker_ci_cd_evidence",
    "cocotb_evidence",   // ← new
]),
"enterprise" => Ok(&[
    "manifest_contract",
    "source_portability",
    "open_source_tool_evidence",
    "wrapper_package_compatibility",
    "docker_ci_cd_evidence",
    "cocotb_evidence",   // ← new, inherited
    "vendor_tool_evidence",
    "board_hardware_evidence",
    "release_support_legal_evidence",
]),
```

Refuse to add the new row to `community`:

```
AF_EVIDENCE_ROW_COMMUNITY_FORBIDDEN: community tier is intentionally minimal (manifest_contract + source_portability). Add to verified-package or enterprise instead.
```

### Step 5 — wire docs

In `docs/licensing.md::Commercial tiers` there is a Markdown table with three
rows (`community`, `verified-package`, `enterprise`) and a cell describing the
required evidence rows. Update the matching cells to include the new row name.
Maintain alphabetical or logical order within each cell — match the existing
style.

### Step 6 — wire unit test (`crates/af-report/src/lib.rs::tests`)

Add a test pattern. Two test bodies — supported and blocked:

```rust
#[test]
fn <area>_supported_when_<trigger>() {
    let manifest = sample_manifest();
    let artifacts = vec!["<artifact triggering the matcher>".to_string()];
    let warnings: Vec<String> = vec![];
    let limitations: Vec<String> = vec![];
    let report = reusable_core_maturity(&MaturityInputs {
        manifest: Some(&manifest),
        artifacts: &artifacts,
        warnings: &warnings,
        limitations: &limitations,
        ci_evidence: &[],
        current_commit_sha: None,
        placeholder_boards: &[],
    });
    let row = report.rows.iter().find(|r| r.area == "<area>").unwrap();
    assert_eq!(row.status, "supported");
}

#[test]
fn <area>_blocked_when_absent() {
    let manifest = sample_manifest();
    let report = reusable_core_maturity(&MaturityInputs {
        manifest: Some(&manifest),
        artifacts: &[],
        warnings: &[],
        limitations: &[],
        ci_evidence: &[],
        current_commit_sha: None,
        placeholder_boards: &[],
    });
    let row = report.rows.iter().find(|r| r.area == "<area>").unwrap();
    assert_eq!(row.status, "blocked");
    assert!(row.limitations.iter().any(|l| l.contains("<keyword from limitations>")));
}
```

If `sample_manifest()` does not exist in the tests module, build a minimal
`CoreManifest` inline or use one of the existing test helpers.

### Step 7 — wire integration test (only if `tier_membership` is non-empty)

In `crates/af-cli/tests/cli.rs`, add a test ensuring the existing
`af-reset-sync` example (or the most basic example) **fails** the affected tier
specifically because of the new row:

```rust
#[test]
fn <area>_blocks_<tier>_tier_on_reset_sync() {
    let root = repo_root();
    let build = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.current_dir(&root)
        .arg("--build-root")
        .arg(build.path())
        .args(["core", "verify", "examples/af-reset-sync", "--tier", "<tier>", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"area\": \"<area>\""));
}
```

This anchors the new row to a stable example and proves the tier mapping is
wired correctly.

### Step 8 — compile + test

```bash
cargo build --workspace
cargo test -p af-report
cargo test -p af-cli --test cli <area>_      # filter to new tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

If any step fails, surface the failure and stop. Do not "fix" by deleting the
new row — that erases the patch.

### Step 9 — pre-existing examples sanity

```bash
cargo run --quiet -p af-cli --bin af -- core verify examples/af-reset-sync --tier community --json
```

`community` tier must still pass (its mapping was not changed). If it fails, the
row's matcher is unexpectedly catching `examples/af-reset-sync` for `community`
— back out and review the matcher substrings.

For `verified-package`/`enterprise`, the test in step 7 already covers the
expected new failure.

## Required output

```
## Added evidence row `<area>`

Touch-points:

- `crates/af-report/src/lib.rs:<line>` — row builder
- `crates/af-cli/src/main.rs:<line>` — tier_required_rows ({tiers})
- `docs/licensing.md:<line>` — Commercial tiers table
- `crates/af-report/src/lib.rs:<line>` — unit tests (supported/blocked)
- `crates/af-cli/tests/cli.rs:<line>` — integration test on tier {tier}

Evidence source: <artifact_substrings|manifest_field|custom>
Triggers: <comma-list of substrings or field path>
Tier membership: <none|verified-package|enterprise>

## Smoke

- `cargo build --workspace` ✓
- `cargo test -p af-report` ✓
- `cargo test -p af-cli --test cli <area>_*` ✓
- `cargo fmt --check` ✓
- `cargo clippy --workspace --all-targets -- -D warnings` ✓

## Impact on existing examples

- `examples/af-reset-sync` tier `<tier>`: <still passes | now fails on this row, expected>
- `examples/af-pdm-rx` tier `<tier>`: <same>
- `examples/af-mod-add` tier `<tier>`: <same>

## CHANGELOG

A new row in the maturity report is an *additive* change to `AfReport.maturity.rows[]`
(consumers ignore unknown rows). Adding the row to `tier_required_rows` for
verified-package/enterprise IS a breaking change for `af core verify` users
who currently pass — note this in `CHANGELOG.md` under `Unreleased` before
commit. Run `af-cli-contract-guard` to confirm.

## Next

Examples currently affected may need either evidence production
(`af-evidence-refresh`) or registry promotion downgrade (e.g. `maturity =
"experimental"` instead of `"preview"`) if the new gate is genuinely high
maturity.
```

## Test Design Obligation

When this skill modifies `af`, it must add thoughtful tests for the touched
behavior. Cover success, failure, deterministic JSON/error output, and evidence
boundaries where applicable; if no direct test is possible, state the reason and
cite the closest existing coverage.

## Hard rules

- **Never modify an existing row.** This skill only adds. Modifications go
  through `af-cli-contract-guard` as a breaking change.
- **Never add to `community` tier.** Community is intentionally minimal. New
  rows go to `verified-package`, `enterprise`, or stay tier-agnostic.
- **Row name is snake_case, lowercase, no spaces.** Verifying via
  `^[a-z][a-z0-9_]*$`.
- **`matching_artifacts` is case-sensitive substring match.** Pick substrings
  that map to _real_ artifact paths under `.af-build/`. The user must
  demonstrate one if asked.
- **Aggregation rows are off-limits.** `evidence_portability`,
  `buyer_grade_readiness`, `enterprise_grade_readiness` are computed from other
  rows. Do not insert anything that aggregates.
- **All five touch-points (or four when tier-agnostic) or none.** If any
  compile/test step fails, revert.
- **Update `CHANGELOG.md` only with explicit confirmation.** The skill suggests;
  the user writes the entry.
- **Stay within `af-report` and `af-cli`.** No new crates, no library
  reshuffles.

## Edge cases

| Situation                                                          | Treatment                                                                                                                                                       |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Existing example would silently lose tier eligibility              | Always flag in the "Impact" section. The user decides whether to downgrade the example's `maturity` or produce real evidence.                                   |
| User wants the row tier-agnostic                                   | Skip step 4 and 7; the row appears in `af core report` but no tier requires it (informational only).                                                            |
| User wants the row in multiple tiers                               | Add to the most permissive (`verified-package`) and let inheritance carry it to `enterprise`.                                                                   |
| User wants a `planned` default instead of `blocked`                | Use the `planned` branch in the row builder; `planned` indicates "work declared, evidence pending".                                                             |
| `matching_artifacts` already matches existing rows' substrings     | Pick more specific substrings; substring collisions create false-supports for unrelated rows.                                                                   |
| The new row should also pull from `manifest.verification_required` | That is a different pattern — declare a kind in `VerificationKind` enum first, then this row mirrors gate completion. Coordinate with `af-architecture` checks. |

## Example session

User:
`add cocotb_evidence row, triggered by artifacts containing 'cocotb', required by verified-package`.

1. Collision check: `cocotb_evidence` not in `rows.push(...)` inventory. Free.
2. Insertion point: before `vendor_tool_evidence`.
3. Wire row builder with `matching_artifacts(artifacts, &["cocotb"])`.
4. Wire tier mapping: append `"cocotb_evidence"` to `verified-package` and
   `enterprise` arrays.
5. Update `docs/licensing.md` "verified-package" and "enterprise" cells.
6. Two unit tests (supported when `["cocotb_run.log"]`; blocked when empty).
7. Integration test
   `cocotb_evidence_blocks_verified_package_tier_on_reset_sync`.
8. `cargo test -p af-report` and
   `cargo test -p af-cli --test cli cocotb_evidence_` both pass.
9. `examples/af-reset-sync` for `community` still passes; for `verified-package`
   now also fails on `cocotb_evidence` (in addition to existing blockers).

Output as per template, with explicit CHANGELOG suggestion since this is a
breaking change for any consumer currently verifying at `verified-package`.
