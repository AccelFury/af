# Contributing to `af`

`af` is pre-alpha and actively developed. Contributions are welcome — **the
fastest way to influence the project is to open an issue first**. The issue
templates under `.github/ISSUE_TEMPLATE/` cover bugs, feature requests, new IP
requests, new board requests, board bring-up, and "I don't know how to do X".

This document covers the engineering rules a PR must satisfy.

## Core principles

`af` is built around four principles. They override personal preferences when in
conflict.

1. **Manifest-first.** Every claim about a core flows from `af-core.toml` plus
   committed artefacts. Nothing is true just because someone said so in a PR
   description.
2. **Honest readiness.** Reports never claim timing closure, CDC sign-off,
   vendor implementation, or hardware suitability without evidence. Maturity
   rows stay `planned` or `blocked` until the matching artefact is on disk.
3. **Layered RTL.** Generic cores ship as portable Verilog-2001. Vendor
   primitives, encrypted netlists, PLLs / clock managers, AXI-only ports, and
   SystemVerilog-only constructs belong in optional wrappers under
   `vendor/<vendor>/` or `wrapper/<bus>/`, never in `rtl/common/`.
4. **Stable contract.** CLI command names, JSON shapes,
   `AF_<DOMAIN>_<CONDITION>` error codes, exit codes, manifest fields, and
   registry schemas are part of the public API. Removing or renaming any of them
   is breaking and requires a `CHANGELOG.md` entry.

## Before opening a PR

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p af-cli --bin af -- registry check --json
cargo run -p af-cli --bin af -- self check --json
cargo run -p af-cli --bin af -- core check examples/af-mod-add --json
```

If your change touches CLI / JSON / manifest / registry surfaces, also run the
contract guard (manual or via the bundled skill):

```bash
git diff HEAD -- \
  crates/af-cli/src/main.rs crates/af-cli/src/commands/ \
  crates/af-manifest/src/lib.rs crates/af-complexity/src/lib.rs \
  crates/af-report/src/lib.rs crates/af-cli/src/cores_registry.rs \
  schemas/ registries/cores.registry.json \
  docs/cli-reference.md docs/licensing.md docs/manifest-reference.md
```

Anything non-empty in that diff means you should walk the checklist in
`.claude/skills/af-cli-contract-guard/SKILL.md` (CLI flags, JSON shapes, error
codes, exit codes, manifest schema, registry, tier mapping).

### Optional: lefthook pre-commit hook

If you have [lefthook](https://github.com/evilmartians/lefthook) installed
(`brew install lefthook` on macOS, `apt install lefthook` on recent
Debian/Ubuntu, or `go install github.com/evilmartians/lefthook@latest`):

```bash
lefthook install
```

This wires `.claude/skills/af-cli-contract-guard/check.sh` to run before every
commit, plus `cargo fmt --check` and `cargo clippy --workspace`. The hook is
local to your clone; the repo does not enforce it in CI (CI runs the same checks
independently). Skip with `LEFTHOOK=0 git commit ...` when you need to bypass it
for an in-progress branch.

Optional Deno checks, if Deno is installed:

```bash
deno fmt --check
deno lint
deno test --allow-read
deno task audit:repo
```

## When you add a new CLI subcommand

The four touch-points (rule #5 in [CLAUDE.md](CLAUDE.md)) must move together in
the same PR:

1. Clap enum branch in `crates/af-cli/src/main.rs`.
2. `*_command_name` lifecycle function in `crates/af-cli/src/main.rs`.
3. New line in `docs/cli-reference.md`.
4. Integration test in `crates/af-cli/tests/cli.rs`.

The structural scaffolder at `.claude/skills/af-add-subcommand/SKILL.md`
produces all four with a TODO-stub handler that already returns
`AF_<NAMESPACE>_<NAME>_UNIMPLEMENTED`. Implement the body afterwards.

## When you add a new evidence row

A new row in `ReusableCoreMaturity` (`crates/af-report/src/lib.rs`) touches four
places too:

1. `reusable_core_maturity` row builder.
2. `tier_required_rows` in `crates/af-cli/src/main.rs` (only if the row is
   required for `verified-package` or `enterprise`; never add to `community`).
3. The `Commercial tiers` table in `docs/licensing.md`.
4. Unit test in `crates/af-report/src/lib.rs::tests` + integration test in
   `crates/af-cli/tests/cli.rs` if the row gates a tier.

Adding a required row to `verified-package` or `enterprise` is a breaking change
for `af core verify` consumers and must be noted in `CHANGELOG.md` under
`Unreleased`. The scaffolder is at
`.claude/skills/af-add-evidence-row/SKILL.md`.

## When you add a new error code

Use the namespace shape `AF_<DOMAIN>_<CONDITION>`. Every error must carry
`code`, `message`, `hint`, `exit_code`. The hint should be at least one full
sentence. Re-use an existing code when the condition matches; never invent
ad-hoc strings.

The `.claude/skills/af-error-explainer/test.sh` regression test enumerates every
`AF_*` code under `crates/` and asserts a real origin and a real hint within ±15
lines. Run it after touching error code declarations:

```bash
.claude/skills/af-error-explainer/test.sh
```

## When you add or change a core in `examples/`

- Use `.claude/skills/af-bootstrap-core/` for new cores (or `af core new`
  directly).
- Use `.claude/skills/af-migrate-manifest/` for legacy v0.1 / v0.2 manifests.
- Update `registries/cores.registry.json` if the core is meant to be part of the
  universal-core inventory; the JSON-schema is at
  `schemas/cores.registry.schema.json` and `af registry check` validates it.
- Register a self-check target in `af-selfcheck.toml` if the core must pass CI
  on every commit.

## When you change vendor / board support

- Board profiles live under `boards/<vendor>/<board>/`. Schema is at
  `schemas/af-board.schema.json`.
- The board matrix in `docs/board_matrix.md` is generated by
  `af board matrix --output docs/board_matrix.md` — regenerate it in the PR.
- Aliases for legacy board IDs live in `registries/board_aliases.json`.

## When you touch documentation

- `docs/cli-reference.md` is the canonical command list. Match the table-style
  block and the JSON contract section.
- `docs/manifest-reference.md` is the canonical manifest spec.
- `docs/licensing.md` is the canonical commercial-tier definition.
- Anything not in those three is an explanatory guide; keep it short and link
  out.

## Style

- Rust: 4-space indent, no tabs. `cargo fmt` is authoritative.
- Markdown: 80-character soft wrap where reasonable. No emoji unless they carry
  verdict meaning (✅ / ❌ / ⚠️ in tier outputs are the only sanctioned uses).
- TOML: prefer table form (`[clocks]`) over array-of-tables (`[[clocks]]`) when
  there is only one entry; keep arrays-of-tables for genuinely repeated
  structures.
- Tests: integration tests in `crates/af-cli/tests/cli.rs` use `assert_cmd` +
  `predicates`. Unit tests live next to the code they test.

## Local development conventions

- Build artefacts must stay under `.af-build/` (or any explicit `--build-root`).
- Local agent configs (e.g. `.claude/settings.local.json`, Cursor / Codex
  caches) are git-ignored. Do not commit them.
- The shared agent/skill set under `.claude/agents/` and `.claude/skills/` IS
  tracked; that is contributor tooling, not per-user state.

## Licensing of contributions

By contributing a PR you agree to the [CLA](CLA.md). Generated reusable IP cores
produced by `af core new` carry the `AccelFury Source Available License v1.0`;
everything else keeps its file-level SPDX header. Do not relicense imported
sources.

## Asking for help

Open an issue with the `Question / "how do I X"` template. State the goal, the
commands you tried, and where you got stuck. The same applies to "I think the
docs are wrong" — that is a documentation bug and absolutely worth reporting.

For bugs, prefer a reproducible report with structured output:

```bash
af doctor --json
af self check --json
af <failing command> --json > error.json
af agent context --from-error error.json
```

Attach the JSON payload and command output to the issue. It lets maintainers
reproduce the exact CLI/report contract instead of guessing from prose.
