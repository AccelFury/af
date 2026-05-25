# SemVer Policy

`af` exposes four independent versioned surfaces. Each follows Semantic
Versioning (`MAJOR.MINOR.PATCH`) with the **breaking ⇒ MAJOR**, **additive
backward-compatible ⇒ MINOR**, **bug-fix-only ⇒ PATCH** discipline. Releases
land under a single `af` workspace version (the CLI version), but each
sub-surface declares its own schema/contract version that may move on a
different cadence.

> **Pre-1.0 note.** `af` is currently pre-`0.x`. Until `1.0`, **MINOR** bumps
> may include intentional breakage when explicitly documented in `CHANGELOG.md`.
> The shapes of evidence rows and exit codes are stable across `0.x`; manifest
> and report schemas evolve via the explicit version fields below.

---

## 1. Public surfaces (what counts as the contract)

The following are part of the public contract. Removing, renaming, or
silently changing the meaning of any item below is **breaking**.

- `af` CLI subcommands, flags, positional arguments, and their meanings
  (`docs/cli-reference.md`).
- Exit codes (`docs/cli-reference.md` — 0, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12).
- Error codes (`AF_<DOMAIN>_<CONDITION>`) — the `code` field of every
  structured error and every JSON error payload.
- JSON output shapes: keys, types, enum values, and required vs. optional
  status. Covered by the report schema (see §3).
- Manifest fields in `af-core.toml` (`crates/af-manifest/src/lib.rs` +
  `schemas/af-core.schema.json`). Covered by `af_version` (see §2).
- Registry shapes: `registries/cores.registry.json`,
  `registries/boards.registry.json`, and their schemas under `schemas/`.
- Manifesto axes: `portability_level`, `priority`, `maturity`,
  `verification_required`. Their enum values are stable.

Internal-only items — module paths inside `crates/af-*` not re-exported,
private helpers, log lines on stderr, scratch files under `.af-build/` —
are **not** part of the contract.

---

## 2. Manifest schema (`af-core.toml` → `af_version`)

The manifest carries an `af_version = "0.x"` string. Today: `"0.1"`, `"0.2"`,
`"0.3"`, `"0.4"` are accepted. Policy:

- **PATCH-equivalent** (no version bump): clarifying defaults, expanding
  allowed values of an enum, fixing typos in error messages. No `af_version`
  change required.
- **MINOR-equivalent** (add to the in-place version, no bump): adding a new
  **optional** field with a sensible default, adding a new optional table.
  Bump `schema_version` in this document and add a CHANGELOG entry. Existing
  manifests must continue to parse and validate.
- **MAJOR-equivalent** (new `af_version`): renaming or removing a field,
  changing a field's type, tightening a previously-allowed shape, making an
  optional field required, narrowing an enum. The previous `af_version`
  string continues to be accepted for at least one full minor release of the
  workspace, with a deprecation note in `CHANGELOG.md`.

When a new `af_version` lands, both the Rust struct
(`crates/af-manifest/src/lib.rs`) and the JSON Schema
(`schemas/af-core.schema.json`) must be updated together, and
`docs/manifest-reference.md` must describe the diff.

---

## 3. Report schema (`AfReport.schema_version` and `report_version`)

Every JSON report carries two version strings:

- `schema_version` — the structural shape of the envelope (top-level keys,
  reproducibility block, `command_payload` discriminator).
- `report_version` — the per-command payload shape (the fields inside
  `command_payload` for a given `kind`).

Adding a new optional field to a payload is **MINOR**: bump `report_version`
only and note it in `CHANGELOG.md`. Renaming or removing an existing field,
changing a type, or changing the `kind` discriminator is **MAJOR**: bump both
`schema_version` and `report_version`, and gate the change behind a
CHANGELOG entry that names a migration path.

A new `kind` is **MINOR** (additive). Removing a `kind` is **MAJOR**.

---

## 4. Core SemVer (the `version` field of a published core)

Authors of reusable cores apply SemVer to the **external** behavior visible
to integrators — not to internal refactors.

- **MAJOR** — any change to the external contract:
  - port name, direction, width, or clock domain;
  - parameter name removed; parameter default narrowed; parameter range
    tightened;
  - reset polarity or async/sync semantics;
  - protocol on a bus interface;
  - pipeline latency on a port if it is part of the documented contract;
  - any change that breaks bit-accuracy against a previously published
    reference.
- **MINOR** — new optional ports/parameters with safe defaults, new backend
  support, performance improvements that do not change observable behavior,
  new evidence rows that flip from `planned` to `supported`.
- **PATCH** — bug fixes that restore documented behavior, doc fixes, build
  fixes, vector regeneration that does not change values.

A `CHANGELOG.md` entry is required for every published core release.

---

## 5. Error codes (`AF_<DOMAIN>_<CONDITION>`)

- **Adding** a new code is **MINOR**. The new code must be documented in
  `docs/cli-reference.md` and accompanied by a CHANGELOG entry.
- **Renaming** or **removing** a code is **MAJOR** — agents and CI systems
  match on these strings. Provide a deprecation alias for at least one minor
  release when feasible.
- **Repurposing** a code (changing what condition it represents) is **MAJOR**
  even if the spelling is unchanged.

Exit codes (the integer process exit) are stable: see the table in
`docs/cli-reference.md`. New conditions should reuse an existing exit-code
slot when the domain matches; introducing a new exit code is **MAJOR**.

---

## 6. Workflow

Every change touching the surfaces above:

1. Identify which surface is changing and pick MAJOR / MINOR / PATCH.
2. Write the CHANGELOG entry under `## Unreleased` first. Include the new
   code/field/flag and the migration note for breaking changes.
3. Bump `schema_version` / `report_version` / `af_version` only when the
   policy above requires it. Adding optional fields does **not** require an
   `af_version` bump.
4. Run the contract guard before commit:
   ```bash
   .claude/skills/af-cli-contract-guard/check.sh
   ```
   It refuses commits that touch CLI/JSON/manifest/registry surfaces without
   a matching CHANGELOG entry and/or version bump.
5. Add or update a focused test for the changed behavior. If no direct test
   is possible, name the closest existing coverage in the PR description.

---

## 7. Cross-references

- `CHANGELOG.md` — user-visible diff for every release.
- `docs/cli-reference.md` — authoritative list of subcommands, flags, exit
  codes, and `AF_*` error codes.
- `docs/manifest-reference.md` — authoritative `af-core.toml` field guide.
- `schemas/af-core.schema.json` and `schemas/af-report.schema.json` — the
  machine-checkable contract.
- `.claude/skills/af-cli-contract-guard/` — automated pre-commit guard.
