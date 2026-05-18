# Agent workflow for `af`

This document defines the contract between **LLM / AI agents** that drive
`af` from the outside (Claude Code, Codex, Cursor, custom CLI agents,
CI orchestrators) and the tool itself. If you are a human user, read
[README.md](../README.md) and [CONTRIBUTING.md](../CONTRIBUTING.md)
instead — they cover the same ground without the automation framing.

## Who this is for

- Coding agents that invoke `af` programmatically and parse its output.
- Issue-triage agents that watch for `af` failures and report them.
- Per-PR review agents that need to check whether a change touched the
  CLI / JSON / manifest / registry contract.
- Long-running automation that aggregates `af` evidence across many
  cores.

If your code calls `af` more than three times per session, this
document applies to you.

## The contract

These rules are **mandatory** — they are what makes `af` reliable as a
deterministic backend. Breaking them produces failures `af` cannot
diagnose for you.

1. **Always invoke with `--json`.** Human stderr is advisory only.
   Never parse it.
2. **Always set `--build-root` to a writable per-session path.** The
   default `.af-build/` is fine when running from a fresh tree; share
   nothing across sessions to keep artefacts isolated.
3. **Never modify `af-core.toml` mid-run.** If you do, re-run
   `af manifest validate <path> --json` before any subsequent
   `af core *` command.
4. **Treat exit codes per [`docs/cli-reference.md`](cli-reference.md).**
   Specifically: do **not** retry on `AF_BACKEND_UNAVAILABLE` (exit
   code 4). It is environmental — `verilator`/`yosys`/`sby` etc. are
   not installed. Either install them via `af tooling ensure`, or
   accept the row stays `planned` in the report.
5. **Never claim signoff `af` did not return.** If `af core report`
   does not have a `supported` row, you cannot claim that row in
   downstream summaries. This is the same `evidence-first` rule that
   applies to humans.
6. **`AF_AGENT_NAME` env var conveys identity.** Set it to a stable
   identifier (e.g. `claude-code/sonnet-4.6` or `ci-triager-v1`) so
   issue bodies, logs, and audit trails can attribute output to you.

## Standard invocation pattern

```text
1. capture = af doctor --json
   # Records the toolchain you actually have, so failure reports later
   # carry the correct environment_hash.

2. result = af <command> ... --json --build-root <session_dir>
   # Run whatever the user asked for. Always with --json.

3. if result.exit_code != 0:
       # 3a. Understand the failure.
       Invoke the `af-error-explainer` subagent (or equivalent) with
       the structured payload to get a 1–3 step diagnosis.

       # 3b. Decide if it's worth filing.
       Apply the "When to file" checklist below.

       # 3c. If yes, prepare the issue.
       af agent issue --kind <best-fit> \
                      --title "<short, prefixed>" \
                      --summary "<one sentence>" \
                      --from-error <result-json-file> \
                      --output <session_dir>/issue.md \
                      --json

       af agent gh-url --kind <best-fit> \
                       --title "<same title>" \
                       --body-file <session_dir>/issue.md \
                       --json
       # OR:
       af agent gh-cli --kind <best-fit> \
                       --title "<same title>" \
                       --body-file <session_dir>/issue.md \
                       --json

       # 3d. Present the URL or command line to the operator.
       Do NOT POST to GitHub yourself. The agent CLI never submits;
       the human operator (or the agent in an explicit submission
       step) runs `gh issue create` or opens the pre-filled URL.
```

## When to file an issue (and when not)

**File** when:

- A recurring failure (≥2 separate sessions) cannot be diagnosed from
  the existing docs.
- The CLI/JSON contract has a gap: an `AF_<...>` code is documented
  but the hint is wrong, or the command exists but the JSON shape is
  inconsistent across runs.
- A user-facing capability is missing (e.g. a manifest field the user
  expected based on the docs).
- A doc is plainly wrong about current behaviour.

**Do not** file when:

- The failure is a one-off user mistake (typo in `af-core.toml`,
  wrong path).
- The failure matches an entry in [`TODO.md`](../TODO.md) under
  "Recently closed" or active items.
- The failure is `AF_BACKEND_UNAVAILABLE` and the user has not run
  `af tooling ensure` — it is environmental, not a bug.
- The "feature" you want is explicitly listed in
  [`docs/known-limitations.md`](known-limitations.md) — that is by
  design.

When in doubt, use `af agent issue --kind question` instead of `bug`.

## Issue kind selection guide

| Symptom | Kind |
|---|---|
| `af` returned a wrong exit code or wrong JSON shape | `bug` |
| A specific `AF_*` hint pointed at the wrong fix | `bug` |
| You need a CLI flag / subcommand that does not exist | `feature` |
| You need a new evidence row in `ReusableCoreMaturity` | `feature` |
| You need an `af_*` core scaffold not currently in the registry | `ip-request` |
| You need a board not currently in `boards.registry.json` | `board-request` |
| A real board did not bring up (physical fail) | `board-bringup` |
| You don't understand how to do X with `af` | `question` |
| Recurring contract gap that does not fit the others above | `agent-report` |

`agent-report` is the catch-all for issues authored by agents whose
nature does not match a human-oriented template. Use it sparingly;
prefer the specific kind when one fits.

## Required fields

Every issue body produced by `af agent issue` contains a fixed
`## Agent context` block:

- `af_version` — from `env!("CARGO_PKG_VERSION")`.
- `commit_sha` — from `git rev-parse HEAD` at issue-build time.
- `host_os` / `host_arch` — from Rust `std::env::consts`.
- `environment_hash` — FNV1a64 of the sorted tool-version list (same
  rule as `AfReport.reproducibility`).
- `repo` — `<owner>/<repo>` from `git config remote.origin.url`; falls
  back to `AccelFury/af`.
- `working_dir` — current directory at issue-build time.
- `agent_name` — value of `AF_AGENT_NAME` env var (or `"unspecified"`).
- `automated_submission` — always `true`.

Do not strip this block before submission. Maintainers triage on it.

## Duplicate prevention

Before submitting, run a `gh search` query in the same repo:

```bash
gh search issues --repo "$(af agent context --json | jq -r '.repo_owner + "/" + .repo_name')" \
                 "<title keywords>" --state open
```

If the result has ≥1 hit with a similar title or the same `AF_*` code,
**do not create a new issue**. Comment on the existing one with the
new agent context block instead, or leave it alone if the existing
discussion already covers it.

The `af-issue-author` subagent (`.claude/agents/af-issue-author.md`)
implements this dedupe step. Use it when available.

## Submission paths

Three options, in order of preference:

1. **`gh issue create --body-file`** — recommended. Reproducible,
   no URL truncation, machine-attributable via `gh auth`.

   ```bash
   eval "$(af agent gh-cli --kind <k> --title <t> --body-file <p>)"
   ```

   (Or just print the command and let the operator run it.)

2. **Pre-filled URL** — for human-in-the-loop sessions.

   ```bash
   af agent gh-url --kind <k> --title <t> --body-file <p>
   ```

   Open the URL in a browser. GitHub silently truncates body strings
   past ~7500 characters; `af agent` warns in JSON output when that
   threshold is crossed.

3. **Manual paste** — last resort. Copy the contents of
   `af agent issue --output <p>` into the GitHub web form for the
   chosen template.

## What `af agent` will NEVER do

- POST to GitHub.
- Invoke `gh` as a subprocess.
- Read `~/.config/gh/hosts.yml` or any token store.
- Read or modify env vars like `GH_TOKEN` / `GITHUB_TOKEN`.
- Push commits, open PRs, comment on existing issues.
- Fabricate the agent context block (`af_version`, `commit_sha`, etc.
  come from real sources).
- Network-anything. The module is offline-deterministic by design.

Submission is **always** the agent's or operator's explicit action,
done outside this CLI.

## Reference

- `af agent --help` — the canonical command list.
- [`docs/cli-reference.md`](cli-reference.md) — exit codes, JSON
  contract, `af agent` table.
- [`CLAUDE.md`](../CLAUDE.md) — global rules for AI agents in this
  repo.
- [`.claude/agents/af-issue-author.md`](../.claude/agents/af-issue-author.md)
  — subagent that orchestrates the full prepare-and-deduplicate flow.
- [`.claude/agents/af-error-explainer.md`](../.claude/agents/af-error-explainer.md)
  — subagent that turns a structured failure into a fix plan; use
  before deciding to file.
