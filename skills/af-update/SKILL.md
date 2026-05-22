---
name: af-update
description: Use when a canonical AccelFury af CLI, manifest, registry, doc, `.claude` skill/agent, project `skills/af-*`, board, report, generated output, or installed skill mirror changed and derived surfaces may be stale.
---

# AF Update

## Inputs

- canonical surface that changed
- derived surface or installed destination that may be stale
- optional destination: repo-derived outputs, project Codex skills, installed Codex skills, Cursor/Claude skills, or optional runtime bundle if present

## Read Surfaces

- `.claude/skills/**`
- `.claude/agents/**`
- `skills/af-*`
- `docs/**`, especially CLI and board matrix docs
- `docs/agent-workflow.md`
- `boards/**`, `registries/**`
- `Cargo.toml`, `crates/**`
- installed mirrors under `~/.codex/skills/af-*` when the sandbox permits it
- optional runtime surfaces only if they exist in the checkout (`runtime/**`)

## Write Surfaces

- deterministic derived outputs regenerated from canonical sources
- updated docs that describe current CLI or schema behavior
- installed skill sync report or explicit blocked-sync verdict
- refreshed installed skill mirrors when repo-canonical `skills/af-*` changed

## Workflow

1. Identify the canonical source and all derived targets from current repo ownership. Use `docs/agent-workflow.md`, `.claude/skills/**`, `.claude/agents/**`, project `skills/af-*`, and optional runtime registries only when present.
2. Prefer canonical commands for deterministic outputs. Examples: `af board matrix --output docs/board_matrix.md --json`, `af vectors generate --json`, `af wrapper generate ... --json`, or report generation through the CLI command that owns the report.
3. Update hand-authored public docs only to describe existing behavior or behavior changed in the same task. Do not invent capabilities to make docs look current.
4. For project `skills/af-*` changes, compare repo-canonical surfaces against installed `~/.codex/skills/af-*` mirrors when available. Installed homes are derived convenience surfaces, not a second source of truth. Treat `runtime/**` as optional and inspect it only if present.
5. If the current sandbox cannot write `~/.codex/skills` or `~/.cursor/skills`, leave an explicit `blocked-external` or `needs-permission` runtime parity verdict.
6. Record what was refreshed, what source drove it, and which validators were rerun.

## Failure Semantics

- Do not let derived outputs become authoritative.
- Do not manually patch a generated artifact when a deterministic generator exists.
- Do not claim installed skill sync completed when installed home writes are blocked.
- Do not update broad documentation without checking the CLI, schema, or registry behavior it describes.

## Completion Criteria

- Canonical and derived repo surfaces are synchronized or explicitly blocked.
- Installed skill mirrors are refreshed or marked stale with a reason.
- Updated outputs are reproducible from declared commands.
- Relevant validation passes or unrelated known failures are named.

## References

- `docs/agent-workflow.md`
- `docs/cli-reference.md`
- `.claude/skills/**`
- `.claude/agents/**`
- `skills/af-*`
- optional `runtime/**` if present in the checkout
