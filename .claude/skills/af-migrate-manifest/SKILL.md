---
name: af-migrate-manifest
description: Upgrade a legacy `af-core.toml` to the current v0.3 contract and seed the manifesto axes (`portability_level`/`priority`/`maturity`/`verification_required`) from `registries/cores.registry.json` when an entry exists. Use when a core's manifest is older than v0.3, when v0.3 axes are missing, when the user says "migrate <core>", "upgrade af-core.toml", "bring this core to current schema", or when `af core check` reports `AF_MANIFEST_VERSION_UNSUPPORTED`. Do NOT use to create a new core — that is `af-bootstrap-core`.
allowed-tools: Bash, Read, Edit, Write, Glob
---

# af-migrate-manifest

`af manifest migrate` is the built-in subcommand for the supported v0.1 → v0.2 compatibility migration. This skill wraps that exact CLI when needed, then adds the v0.3 axes step that the built-in command does **not** do, and proves the migrated core still passes `af core check` and `af architecture check`. It is the pair-tool to `af-bootstrap-core`: bootstrap creates fresh, migrate brings legacy in line.

## When to invoke

User says or implies:

- "migrate `<core_dir>`"
- "upgrade af-core.toml to v0.3"
- "bring `<core>` to current schema"
- "fix `AF_MANIFEST_VERSION_UNSUPPORTED`"
- "this manifest is old"

Do **not** invoke when the manifest is already v0.3 and already declares all four manifesto axes; just confirm and stop.

## Required inputs

1. **`core_dir`** — directory containing `af-core.toml`. Required.
2. **`write`** — `true` by default. If `false`, dry-run mode: skill only proposes the migration and never modifies files.
3. **Optional override** — `--priority`, `--portability-level`, `--maturity` if the registry has no matching entry **and** the user wants explicit values. Otherwise the skill prompts once.

## Procedure

### Step 1 — establish current state

```bash
cargo run --quiet -p af-cli --bin af -- manifest validate <core_dir>/af-core.toml --json
```

Read the JSON. Extract:

- `manifest.af_version` (could be `"0.1"`, `"0.2"`, or `"0.3"`).
- `manifest.name` and `manifest.core` (used to look up the registry entry).
- Whether `manifest.portability_level`, `manifest.priority`, `manifest.maturity` are `null` and whether `manifest.verification_required` is empty.

If `af_version == "0.3"` AND all four axes are populated, stop with:

```
## Already up to date

`<core_dir>/af-core.toml` is v0.3 with all manifesto axes set. Nothing to migrate.
```

### Step 2 — normalise v0.1/v0.2 shape via `af manifest migrate`

If `af_version` is `"0.1"`, run the currently supported migration:

```bash
cargo run --quiet -p af-cli --bin af -- manifest migrate \
  --from 0.1 --to 0.2 <core_dir>/af-core.toml --write --json
```

This parses the manifest through `CoreManifest::from_path`, normalizes supported
legacy shapes, sets `af_version = "0.2"`, and rewrites:

- legacy `[name]` table → root-level `name`/`vendor`/`library`/`core`/`version`
- legacy `[[sources]]` array → `[sources]` table with `files`/`include_dirs`/`roles`/`file_types`
- legacy `[[formal]]` array → `[formal]` table
- legacy `[[boards]]` array → `boards = [...]` string array
- legacy `[[backend_compatibility]]` array → `[backend_compatibility]` boolean flags
- legacy `[[known_limitations]]` array → `known_limitations = [...]` string array

It does **not** bump `af_version` to `"0.3"` and does **not** set manifesto axes — that is step 3 and 4.

If `--write` is `false` (dry-run), instead run without `--write`; the command writes `af-core.toml.migrated-0.2.toml` next to the original. Read it from there for the next steps. Do not delete the original yet.

If `af_version` is `"0.2"` or `"0.3"` but any manifesto axis is missing, skip
`af manifest migrate`; the built-in CLI does not support `0.2 -> 0.3`.
Proceed directly to registry lookup and the metadata-only axes step.

### Step 3 — look up manifesto axes from the cores registry

Find the registry (try `<core_dir>/../../registries/cores.registry.json`, then `<repo_root>/registries/cores.registry.json` via `git rev-parse --show-toplevel` if inside the `af` repo). If not present (skill invoked outside the `af` repo), skip to manual prompt in step 4.

```bash
jq --arg id "<manifest.core>" '.cores[] | select(.core_id == $id)' \
   <registry_path>
```

If the entry exists, capture: `category`, `priority`, `portability_level`, `maturity`, `verification_required[]`. These are the canonical values.

If the entry does NOT exist:

- Ask the user **once**: "no registry entry for `<core>`. Provide manifesto axes (or `default`): priority=?, portability_level=?, maturity=?".
- `default` means: `priority = P2`, `portability_level = U0`, `maturity = experimental`, `verification_required = [{kind = "simulation"}]`. Use these only with explicit `default` confirmation.

### Step 4 — apply axes + bump `af_version`

Read the (now-migrated) `af-core.toml`. Parse as TOML. Modify in-place:

1. Set `af_version = "0.3"`.
2. If absent or empty, set `category = "<registry.category>"`.
3. If absent, set `portability_level = "<registry.portability_level>"`.
4. If absent, set `priority = "<registry.priority>"`.
5. If absent, set `maturity = "<registry.maturity>"`.
6. If `[[verification_required]]` is empty and registry lists gates, write each as a `[[verification_required]]` entry with `kind = "<gate>"` and a short auto-description: `"declared in cores.registry.json"`.

Do **not** overwrite existing axis values — if the user already set `priority = "P0"` and the registry says `P2`, keep `P0` and emit a warning in the output report (registry/manifest divergence; user must reconcile manually).

Write the file back.

### Step 5 — clean up scratch artefacts

If dry-run mode produced `af-core.toml.migrated-0.2.toml`, keep it and report
the path. In write mode the source manifest is overwritten and no scratch file
is expected. If a stale scratch file already exists, remove it only when its
content matches the now-current `af-core.toml`; do not remove divergent files.

### Step 6 — prove the migration

Run both:

```bash
cargo run --quiet -p af-cli --bin af -- core check <core_dir> --json
cargo run --quiet -p af-cli --bin af -- architecture check <core_dir> --json
```

Expected:

- `core check`: `"status": "passed"`.
- `architecture check`: `"status": "warning"` is OK — declared `[[verification_required]]` gates without `evidence = ...` produce `AF_VERIFICATION_EVIDENCE_PLANNED` warnings, and that is the desired post-migration state.

Hard failures → roll back nothing (the user owns their tree) but surface the error verbatim and hand off to `af-error-explainer`. Do not attempt heuristic fixes.

## Required output

```
## Migrated `<core_dir>/af-core.toml`

- Version: `<old_af_version>` → `0.3`
- Manifesto axes (sourced from `registries/cores.registry.json` <entry_or_default_or_user>):
  - `portability_level`: <U?>
  - `priority`: <P?>
  - `maturity`: <experimental|preview|beta|stable|deprecated>
  - `verification_required`: <kind, kind, ...>
- Normalisations applied: <bulleted list of which `normalize_*` paths fired>
- Scratch removed: <`af-core.toml.migrated-0.2.toml` | none>

## Post-migration checks

- `af core check`: <passed | failed (code)>
- `af architecture check`: <passed | warning ({N} AF_VERIFICATION_EVIDENCE_PLANNED) | failed (code)>

## Divergence warnings (only if non-empty)

- <"existing `priority = P0` in manifest kept; registry says `P2` — reconcile manually">

## Next gates

1. Generate evidence for the declared verification gates (e.g. `af core sim --backend verilator <core_dir>`).
2. Run `af core verify --tier community <core_dir>` to confirm baseline tier still holds.
```

## Test Design Obligation

When this skill modifies `af`, it must add thoughtful tests for the touched
behavior. Cover success, failure, deterministic JSON/error output, and evidence
boundaries where applicable; if no direct test is possible, state the reason
and cite the closest existing coverage.

## Hard rules

- **No new manifesto axes invention.** Always source from `registries/cores.registry.json`; ask the user only if no entry exists.
- **Never overwrite existing axis values.** Manifest is the user's authored source; divergence is a warning, not an auto-fix.
- **Do not touch `[sources]`, `[ports]`, `[clocks]`, `[resets]`, `[[parameters]]`, `[rtl]`.** Migration is metadata-only. If `af manifest migrate` reshaped these arrays into tables (legacy form), that is the only structural change and it is the built-in's responsibility.
- **No RTL rewrites.** Even if `af core check` fails post-migration with a portable-policy violation, this skill stops and delegates to `af-debug-portable-violation`.
- **Dry-run mode produces no writes.** If `write = false`, output the proposed diff and stop.
- **Do not delete the original manifest under any circumstance.** Only the `.migrated-0.2.toml` scratch file may be removed, and only after content match.
- **No vendor-tool runs.** Migration must succeed with `af doctor` showing only baseline tooling.

## Edge cases

| Situation | Treatment |
|---|---|
| `af_version` is already `0.3` but axes empty | Skip step 2; do steps 3–6 only |
| Manifest is v0.2 in canonical form but missing axes | Do not run `af manifest migrate`; proceed directly to registry lookup and axes insertion. |
| Registry entry has `reference_path` pointing elsewhere | Do not redirect; the user's `<core_dir>` is authoritative |
| Two registry entries collide (should not happen — `cores_registry::check` rejects duplicates) | Stop, surface `AF_CORES_REGISTRY_DUPLICATE_ID`, do not migrate |
| Manifest declares manifesto axes that contradict registry | Keep manifest; emit divergence warning |
| `--write` on a read-only filesystem | Surface OS error; do nothing |
| Skill invoked outside the `af` repo (no registry available) | Ask user for axes via `default` shortcut or explicit values |

## Example session

User: `migrate examples/af-pdm-rx`

1. `af manifest validate examples/af-pdm-rx/af-core.toml --json` → `af_version = "0.1"`, no axes.
2. `af manifest migrate --from 0.1 --to 0.2 examples/af-pdm-rx/af-core.toml --write --json` → canonical v0.2 shape written.
3. `jq '.cores[] | select(.core_id == "af_pdm_rx")' registries/cores.registry.json` → `portability_level = "U0"`, `priority = "P2"`, `maturity = "preview"`, `verification_required = ["simulation", "board-demo"]`.
4. Edit `af-core.toml`: set `af_version = "0.3"`, `category = "audio"`, axes from step 3.
5. `core check` → passed. `architecture check` → warning (2× `AF_VERIFICATION_EVIDENCE_PLANNED`).

Output mirrors the canonical template above.
