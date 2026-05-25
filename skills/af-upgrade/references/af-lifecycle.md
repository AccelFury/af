# AF Lifecycle Reference

## Surface Classes

- `tool-code`: Rust workspace source, crate manifests, integration tests, and
  CLI implementation. Canonical source of behavior.
- `public-cli-contract`: `README.md`, `docs/cli-reference.md`, command examples,
  stable exit codes, JSON output shape, and error codes.
- `manifest-schema`: `crates/af-manifest`, manifest reference docs, migration
  behavior, and manifest fixtures.
- `generated-derived`: `.af-build/**`, `target/**`, wrapper outputs, generated
  reports, vectors, and deterministic docs such as board matrix output.
- `agent-governance`: `docs/agent-workflow.md`, `.claude/agents/**`,
  `.claude/skills/**`, `CLAUDE.md`, and root `AGENTS.md`.
- `history-archive`: `CHANGELOG.md`, release process notes, reports that capture
  past evidence, and provenance docs.
- `example-core`: `cores/**` and `examples/**` used as fixtures, product
  examples, or imported cores.
- `board-registry`: `boards/**`, `registries/**`, `toolchains/**`, and generated
  board matrix docs.
- `report-evidence`: `reports/**` persistent evidence and generated report
  contracts.
- `codex-skill-golden`: repo-canonical installable Codex skills under
  `skills/af-*`.
- `skill-mirror`: installed Codex mirrors under `~/.codex/skills/af-*`.
- `optional-runtime`: runtime deployment surfaces only if a checkout actually
  contains `runtime/**`.

## Retention Verdicts

- `retain-active`: keep as active source of truth.
- `retain-knowledge-base`: keep because it remains reusable reference knowledge.
- `summarize`: replace verbose stale detail with compact current summary.
- `link-out`: replace duplicated detail with a link to the authoritative
  surface.
- `archive`: move from active surface to historical record.
- `trim`: remove obsolete bloat while preserving the active claim.
- `delete`: remove only when obsolete, non-authoritative, and not needed for
  traceability.
- `leave-untouched`: no repair needed in the current scope.

## Blocker Classes

- `auto-fixable`: can be repaired with current files and tools.
- `needs-tooling`: requires optional backend, formatter, simulator, Docker, or
  host tool.
- `needs-design-change`: requires product or architecture decision.
- `needs-user-confirmation`: multiple valid owners or retention choices exist.
- `blocked-external`: requires credentials, network, hardware, permissions, or
  out-of-sandbox installed-skill writes.
- `known-existing-failure`: validation failure reproduced outside the current
  change and not caused by this task.

## Validation Matrix

- Command shorthand: `af <args>` means an installed `af` binary when available,
  otherwise `cargo run -p af-cli --bin af -- <args>` from the repo root.
- Rust formatting: `cargo fmt --all -- --check`.
- Workspace tests: `cargo test --workspace` for broad changes.
- CLI integration: `cargo test -p af-cli --test cli` when command behavior
  changes.
- Manifest behavior: `cargo test -p af-manifest --lib` and
  `af manifest validate <path> --json`.
- Core package behavior: `af core check <core_dir> --json`.
- Board registry: `af registry check --json` and
  `af board matrix --output docs/board_matrix.md --json` when board surfaces
  change.
- Wrapper behavior: targeted `af wrapper generate ... --target <target> --json`
  tests when wrapper surfaces change.
- Agent and skill freshness: `bash scripts/check-af-skills.sh`, plus any
  targeted skill self-test listed by that script.

## Ownership Rules

- `crates/**` owns behavior. Docs and reports must follow crate behavior, not
  the reverse.
- `docs/cli-reference.md` owns public CLI examples and stable error contract
  prose.
- `docs/agent-workflow.md`, `.claude/agents/**`, and `.claude/skills/**` own
  active Claude agent workflow and guidance.
- `skills/af-*` owns the project golden standard for installable Codex `af-*`
  skills.
- `TODO.md` owns the active tool backlog; close items only after acceptance
  criteria are implemented and validated.
- Installed Codex skills are mirrors/convenience surfaces; keep them aligned
  with project `skills/af-*` when writable.
- Generated outputs must name the command or crate that regenerates them.
- Ignored internal archives are historical material, not active source of truth.
