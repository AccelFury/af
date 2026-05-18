---
name: Automated agent report
about: Issue submitted by an LLM/AI agent via `af agent issue`. Humans should use bug_report.md or feature_request.md instead.
labels: agent-generated
---

<!-- Generated body comes from `af agent issue --kind agent-report ...`. -->
<!-- Read docs/agent-workflow.md before submitting; do not strip the   -->
<!-- "Agent context" block — it is the triage contract.                -->

## Summary

<!-- One sentence: what `af` invocation or workflow produced this. -->

## Reproduction

```bash
# Exact commands the agent ran (always with --json).
```

## Structured failure (if any)

<!-- Paste the `--json` payload from the failing command, or omit the
     block entirely if there was no structured failure. -->

```json
```

## Agent context

- `af_version`:
- `commit_sha`:
- `host_os` / `host_arch`:
- `environment_hash`:
- `repo`:
- `working_dir`:
- `agent_name`:
- `automated_submission`: `true`

## Why this is being filed by an agent

<!-- One paragraph: why automation surfaced it — recurring failure,
     contract gap, missing capability, doc lie, etc. -->

## Suggested next step (optional)

<!-- One paragraph: minimal change that would resolve the issue, or a
     specific question to a maintainer. -->
