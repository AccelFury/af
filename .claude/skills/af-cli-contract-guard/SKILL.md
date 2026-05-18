---
name: af-cli-contract-guard
description: Run before any commit/PR that touches `af`'s public surface (CLI commands, JSON shapes, error codes, schemas, exit codes, manifest fields). Compares the staged tree against the HEAD baseline and reports breaking changes that require a coordinated version bump and CHANGELOG entry. Use when the user asks "is this PR safe", "am I breaking the contract", "review my af changes for stability", or proactively before `git commit` on substantive CLI changes. Do NOT use for purely internal Rust refactors that touch zero public symbols.
allowed-tools: Bash, Read, Grep, Glob
---

# af-cli-contract-guard

`af` ships a small, stable contract surface: command names + flags, JSON shapes (`AfReport.schema_version`, `command_payload.kind`, evidence-row areas), error codes + exit codes, manifest fields, registry schemas. Breaking any of these without coordinated bumps will silently break consumers (`fpga-skillls`, CI, scripts). This skill catches the regression before it lands.

## When to invoke

User says or does:

- "review my changes before commit"
- "am I breaking anything"
- "is this safe to merge"
- proactive: after the user finishes a series of edits in `crates/af-cli/src/`, `crates/af-manifest/src/`, `crates/af-report/src/`, `schemas/`, or `registries/cores.registry.json`.

Do not invoke for changes scoped to:

- backend internals (`crates/af-backend-{verilator,icarus,yosys,...}/src/lib.rs`) where neither argv nor returned `BackendReport` shape changes
- pure docstring / comment edits
- new tests, example RTL, or `.af-build/` paths

## Required inputs

None beyond the repo state. The skill operates against the local git tree:

- staged changes (`git diff --cached`)
- unstaged tracked changes (`git diff`)
- baseline = `HEAD` (or the user's specified base via "compare to main")

## What counts as the contract

These are the surfaces guarded — every one corresponds to a downstream
consumer assumption.

### 1. CLI command and flag surface

- The `clap` derive tree under `crates/af-cli/src/main.rs::Commands` and the
  nested `*Command` enums (`CoreCommand`, `RegistryCommand`, etc.).
- Subcommand renames, removed flags, changed defaults, changed
  `arg(long, ...)` names, changed `value_enum` variants.

### 2. JSON output shapes

- `crates/af-report/src/lib.rs::AfReport` — top-level keys, `schema_version`,
  `report_version`.
- `crates/af-report/src/lib.rs::ReusableCoreMaturity` and `MaturityRow` —
  field names. Adding new row areas is **non-breaking**; removing or
  renaming existing ones is breaking.
- `crates/af-report/src/lib.rs::CommandPayload` variants — `#[serde(tag = "kind")]`
  values (`check`, `lint`, `simulation`, `formal`, `build`, `package`,
  `report`, `tooling`, `doctor`, `flash`) are the dispatch contract for
  consumers.
- `crates/af-complexity/src/lib.rs::ClassificationReport` fields.
- `crates/af-cli/src/cores_registry.rs::CoresRegistryReport` fields.

### 3. Error codes and exit codes

- Every `CliError::new("AF_<...>", ...)` literal in `crates/af-cli/src/main.rs`
  and `code()` functions in every domain crate.
- The exit-code table in `docs/cli-reference.md` (lines starting with
  `- ` `<digit>` `: `).

### 4. Manifest schema

- `crates/af-manifest/src/lib.rs::CoreManifest` field names and `#[serde]`
  attributes.
- `schemas/af-core.schema.json` `properties` keys, `required` array,
  `enum` values.

### 5. Registry schemas and tier mapping

- `schemas/cores.registry.schema.json`.
- `registries/cores.registry.json` (entries can be added; removing or
  renaming a `core_id` is breaking for skills that listed it).
- `crates/af-cli/src/main.rs::tier_required_rows` — adding a required row
  to an existing tier is breaking; removing one is *semantically* breaking
  (cores previously failing might now pass).

### 6. Stable docs

- `docs/cli-reference.md` command block (the inline `bash` listing) and
  the JSON contract section.

## Procedure

### Step 1 — capture the diff scope

```bash
git diff --name-only HEAD -- \
  crates/af-cli/src/main.rs \
  crates/af-manifest/src/lib.rs \
  crates/af-complexity/src/lib.rs \
  crates/af-report/src/lib.rs \
  crates/af-cli/src/cores_registry.rs \
  crates/af-cli/src/commands/ \
  schemas/ \
  registries/cores.registry.json \
  docs/cli-reference.md \
  docs/licensing.md \
  docs/manifest-reference.md
```

If the result is empty, this skill has nothing to check. Report
"no contract-bearing files changed; nothing to guard" and exit.

### Step 2 — for each changed contract surface, run the matching diagnostic

#### CLI surface

```bash
git diff HEAD -- crates/af-cli/src/main.rs crates/af-cli/src/commands/ \
  | grep -E '^[+-][^+-].*(#\[command|#\[arg|Subcommand|enum [A-Z][a-zA-Z]*Command|"AF_)' \
  | head -100
```

Look at each `-` line that disappears or every `=>` arm renamed in the
dispatch match. Flag any:

- removed subcommand name
- removed long-form flag
- changed default value
- changed `value_enum` variant
- changed `command_name`/lifecycle string in the `*_command_name` functions

#### JSON shapes

```bash
git diff HEAD -- crates/af-report/src/lib.rs crates/af-complexity/src/lib.rs \
  | grep -E '^[+-]\s*pub (struct|enum|fn) |^[+-]\s*pub [a-z_]+: |^[+-]#\[serde'
```

Flag any line removal of a public field, struct, or enum variant.
Renames are removals + additions.

#### Error codes

```bash
{ git show HEAD:crates/af-cli/src/main.rs; \
  for f in crates/*/src/lib.rs; do git show HEAD:"$f" 2>/dev/null; done; } \
  | grep -ohE 'AF_[A-Z][A-Z0-9_]+' | sort -u > /tmp/af-codes-head.txt

grep -rohE 'AF_[A-Z][A-Z0-9_]+' crates/ | sort -u > /tmp/af-codes-now.txt

diff /tmp/af-codes-head.txt /tmp/af-codes-now.txt
```

Lines prefixed `<` are removed codes. **Any removed code is breaking.**
Lines prefixed `>` are new codes — non-breaking, but should appear in
`CHANGELOG.md` if user-visible.

#### Exit codes

```bash
git diff HEAD -- docs/cli-reference.md \
  | grep -E '^[+-]\s*- `[0-9]+`'
```

Removed or renumbered exit codes are breaking.

#### Manifest schema

```bash
git diff HEAD -- crates/af-manifest/src/lib.rs schemas/af-core.schema.json \
  | grep -E '^[+-]\s*pub [a-z_]+:|^[+-]\s*"[a-z_]+":|^[+-]\s*"required"'
```

Flag removed `pub field:` or removed schema property.

#### Registry

```bash
git diff HEAD -- registries/cores.registry.json | grep -E '^[+-]\s*"core_id":'
```

Removed `core_id` lines are breaking.

#### Tier mapping

```bash
git diff HEAD -- crates/af-cli/src/main.rs | sed -n '/fn tier_required_rows/,/^}/p'
```

Flag any change to existing tier's array.

### Step 3 — version bump expectation

For every breaking change category, the skill expects exactly one of
these to also be in the diff:

| Breaking surface | Required companion change |
|---|---|
| `AfReport` shape | bump `AfReport::new`'s `schema_version` or `report_version` literal in `crates/af-report/src/lib.rs` |
| `cores.registry.json` schema | bump `schema_version` in both `registries/cores.registry.json` and `crates/af-cli/src/cores_registry.rs::SUPPORTED_SCHEMA_VERSION` |
| `af-core.toml` schema | bump allowed `af_version` set in `crates/af-manifest/src/lib.rs` and `schemas/af-core.schema.json::af_version.enum` |
| Removed CLI command/flag | bump workspace package version in `Cargo.toml` (semver-major if 1.0+; minor if pre-1.0) and add note to `CHANGELOG.md` |
| Removed error code or exit code | `CHANGELOG.md` entry under `Unreleased` describing the removal and migration path |

If the companion change is **not** present, this is the regression the
skill flags.

### Step 4 — sanity run

After the diff analysis, run:

```bash
cargo run --quiet -p af-cli --bin af -- registry check --json
cargo run --quiet -p af-cli --bin af -- self check --json
cargo run --quiet -p af-cli --bin af -- manifest validate examples/af-reset-sync/af-core.toml --json
```

Each must return `"status": "passed"`. If any fail, that is a hard
regression — surface the error code immediately. (The PR may also need
`af-error-explainer` to diagnose.)

## Required output

Always exactly:

```
## Contract guard summary

Changed contract files: <N>
Breaking changes detected: <N>
Companion changes detected: <N>

## Findings

(repeat per finding)

### [BREAKING] <one-line summary>

- Surface: <CLI flag / JSON field / error code / ...>
- Removed in: `<path>:<line>` — `<short quote>`
- Required companion: <e.g. "bump `AfReport.report_version`">
- Companion present? <yes | no — file `<...>` did not change>

### [ADDED] <one-line summary>

- Surface: <...>
- Added in: `<path>:<line>`
- Recommend: add a line to `CHANGELOG.md` under `Unreleased`

(no "no findings" section when none)

## Smoke checks

- `af registry check --json` → <status>
- `af self check --json` → <status>
- `af manifest validate examples/af-reset-sync/af-core.toml --json` → <status>

## Verdict

<one of:>
- ✅ SAFE — only additive changes, smoke green.
- ⚠️ ADDITIVE ONLY (CHANGELOG suggested) — new surface, smoke green, CHANGELOG entry missing.
- ❌ BREAKING WITHOUT COMPANION — at least one breaking change has no required companion bump. Block the commit.
- ❌ SMOKE REGRESSION — at least one smoke check failed (independent of diff).
```

## Hard rules

- **Do not edit anything to "fix" findings.** This skill is read-only.
  Surface the problem; let the user decide.
- **Do not run destructive git ops.** No `git stash`, `git reset`, etc.
- **Do not stage or commit.** Even if all green.
- **No false negatives are acceptable.** If you cannot determine whether
  a change is breaking, mark it `[REVIEW NEEDED]` and explain why.
- **No false positives matter twice.** A flagged additive change should
  be marked `[ADDED]`, not `[BREAKING]`. Adding a new public field is
  not a breaking change for serde JSON consumers (unknown fields are
  ignored) unless the field is `required`.
- **Never claim a smoke check passed if you did not actually run it.**
- **Stay short.** The whole report should fit in one screen unless many
  findings exist.

## Edge cases

| Situation | Treatment |
|---|---|
| User adds a new error code | `[ADDED]`; recommend CHANGELOG line |
| User adds a new evidence row to `ReusableCoreMaturity` | `[ADDED]`; if tier mapping in `tier_required_rows` references it as required, that's a `[BREAKING]` tier change |
| User renames a private helper fn inside `core_new.rs` | not in contract; skip |
| User changes the human-readable string in `CliError::new(..., message, ...)` | not in contract; messages are advisory |
| User changes `hint` text | not in contract |
| User adds a new subcommand | `[ADDED]`; recommend doc update in `docs/cli-reference.md` |
| User bumps `report_version` without any shape change | report it as `[REVIEW NEEDED]` (gratuitous bump) |
| Diff against a base other than `HEAD` (user says "compare to main") | replace `HEAD` with `main` in the git diff commands above |
