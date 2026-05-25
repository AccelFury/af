---
name: af-heal
description: Use when AccelFury af docs, TODOs, agent/skill guidance, reports, manifests, crate APIs, CLI references, board registry data, generated outputs, or installed skill mirrors diverge or look stale.
---

# AF Heal

## Inputs

- drift signal, failing validation, stale generated artifact, or governance
  mismatch
- affected repo surface or command scenario
- optional explicit scope such as `docs`, `crates`, `boards`, `reports`,
  `.claude/skills`, project `skills/af-*`, installed Codex skills, or
  `agent-governance`

## Read Surfaces

- `TODO.md`, `README.md`, `CHANGELOG.md`
- `docs/**`, especially `docs/cli-reference.md`, `docs/dev-roadmap.md`,
  `docs/agent-workflow.md`
- `.claude/agents/**`, `.claude/skills/**`
- `Cargo.toml`, `Cargo.lock`, `crates/**`
- `boards/**`, `registries/**`, `toolchains/**`
- `cores/**`, `examples/**`
- `reports/**`
- project Codex golden source under `skills/af-*`
- installed mirrors under `~/.codex/skills/af-*` when the sandbox permits it
- optional runtime surfaces only if they exist in the checkout (`runtime/**`)

## Write Surfaces

- repaired source, docs, manifests, tests, or governance artifacts
- explicit residual blocker list
- retention verdict matrix for stale material
- validation evidence for touched surfaces
- parity status when project `skills/af-*` or installed skill mirrors changed

## Workflow

1. Classify the target from current repo ownership before editing. Use
   `docs/agent-workflow.md`, `docs/dev-roadmap.md`, and optional freshness
   registries only when present. Decide whether each touched object is
   `tool-code`, `public-cli-contract`, `manifest-schema`, `generated-derived`,
   `agent-governance`, `history-archive`, `example-core`, `board-registry`,
   `report-evidence`, or `skill-mirror`.
2. Identify source of truth and derived surfaces. Do not repair generated
   artifacts by hand when a local command can regenerate them from canonical
   inputs.
3. Inventory every covered object in the declared scope, not only the file that
   first looked stale. Emit a retention verdict for stale material:
   `retain-active`, `retain-knowledge-base`, `summarize`, `link-out`, `archive`,
   `trim`, `delete`, or `leave-untouched`.
4. Separate repair lanes logically: freshness, domain ownership, implementation
   boundary, and validation merge. Use live subagents only when the user
   explicitly asks for delegated/parallel agent work; otherwise perform the same
   lanes serially in the main agent.
5. Repair locally resolvable drift. Route broad product gaps to `TODO.md` or
   `docs/dev-roadmap.md` only when the fix is not feasible in the current task.
6. Keep root governance surfaces about the `af` tool itself. Core-specific
   findings belong with the core, example, report, or agent ledger that produced
   the evidence.
7. If project `skills/af-*` or installed `~/.codex/skills/af-*` mirrors change,
   update the parity verdict or report installed sync as blocked by permissions.
   Treat `runtime/**` as optional and only inspect it if present.
8. Run the narrowest relevant validation first, then broader workspace gates
   when shared behavior changed.

## Failure Semantics

- Do not hide drift by editing only the visible symptom.
- Do not delete authoritative history when archive, summary, or link-out is
  sufficient.
- Do not treat missing optional FPGA tools as RTL failure; classify them as
  `needs-tooling`.
- Do not rewrite append-only agent ledgers unless the schema explicitly permits
  it; append status events instead.
- Do not manually fork installed Codex skills away from repo-canonical
  `skills/af-*`.
- If write ownership or source of truth is unclear, stop with
  `needs-user-confirmation`.

## Completion Criteria

- Locally repairable drift is fixed.
- Remaining blockers are explicit and classified.
- Touched surfaces have validation evidence.
- Generated and canonical surfaces are not confused.
- Skill mirror changes have a parity verdict.

## References

- `docs/agent-workflow.md`
- `docs/dev-roadmap.md`
- `.claude/agents/**`
- `.claude/skills/**`
- `skills/af-*`
- optional `runtime/**` if present in the checkout
