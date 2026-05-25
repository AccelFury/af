---
name: af-upgrade
description: Use when improving AccelFury af tool maturity, reliability, usability, CLI behavior, schemas, reports, backends, tests, docs, governance, or agent operability.
---

# AF Upgrade

## Inputs

- maturity gap from `TODO.md`, `docs/dev-roadmap.md`,
  `docs/testing-strategy.md`, or a concrete audit finding
- current implementation and validation state
- target user outcome for the `af` tool

## Read Surfaces

- `TODO.md`
- `docs/dev-roadmap.md`
- `docs/product-requirements.md`
- `docs/software-requirements-specification.md`
- `docs/technical-design.md`
- `docs/agent-workflow.md`
- `.claude/agents/**`, `.claude/skills/**`
- `skills/af-*`
- `Cargo.toml`, `crates/**`
- `docs/cli-reference.md`, `README.md`, `CHANGELOG.md`
- optional runtime registries only if they exist in the checkout (`runtime/**`)

## Write Surfaces

- scoped `af` product or architecture improvements
- tests that prove the new maturity delta
- docs and CLI references matching changed behavior
- TODO or agent-ledger status updates when a tracked gap is closed
- maturity evidence when the change materially improves the tool

## Workflow

1. Select one maturity gap and state its user-visible outcome before editing.
   Prefer a tracked gap from `TODO.md`, `docs/dev-roadmap.md`,
   `docs/testing-strategy.md`, or a concrete audit finding.
2. Classify affected surfaces from current repo ownership (`crates/**`,
   `docs/**`, `.claude/**`, `skills/af-*`, `registries/**`, `boards/**`). Keep
   tool-wide improvements separate from one-off example-core cleanup unless the
   example is the acceptance fixture.
3. Implement the smallest coherent improvement that changes real behavior,
   diagnostics, automation, or verification depth.
4. Update public interfaces together: CLI args, JSON output, error codes,
   manifest/report schemas, docs, and tests must agree.
5. Add or adjust tests in the crate that owns the behavior. Broaden testing only
   when the change crosses crate or CLI boundaries.
6. Close or update the originating TODO/governance item only after validation
   proves the acceptance criteria.
7. Run `af-heal` logic after the upgrade if the change creates derived-surface
   drift.

## Failure Semantics

- Do not close maturity work with docs-only edits unless the gap was
  documentation-only.
- Do not mix unrelated TODO items into one broad upgrade.
- Do not create a second policy path when an existing crate, doc, or registry
  owns the behavior.
- Do not ignore known test failures; classify unrelated failures separately from
  upgrade regressions.
- Do not weaken structured errors, hints, or JSON contracts to make tests pass.

## Completion Criteria

- The selected maturity gap has a concrete implemented delta.
- Tests cover the changed behavior or diagnostics.
- Public docs and governance state match the new behavior.
- Residual gaps are tracked explicitly and not hidden in prose.

## References

- `docs/dev-roadmap.md`
- `docs/testing-strategy.md`
- `docs/agent-workflow.md`
- `.claude/agents/**`
- `.claude/skills/**`
- `skills/af-*`
- optional `runtime/**` if present in the checkout
