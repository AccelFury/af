# Agent Operating Guide

This directory defines how LLM agents should use and improve the AccelFury
`af` tool.

Files:

- `agent-operating-protocol.yaml`: machine-readable source of truth for agent
  workflows, command scenarios, logging, TODO issue capture, and maturity
  reflection.
- `todo-issues.jsonl`: append-only machine-readable backlog of maturity,
  quality, and value gaps.
- `maturity-ledger.jsonl`: append-only machine-readable self-reflection history.

Agents should prefer `--json` CLI output, record concise evidence, and avoid
free-form undocumented state. Generated or session-specific logs belong under
`reports/agent/logs/`.

Minimum agent loop:

1. Run the scenario from `agent-operating-protocol.yaml` that matches the task.
2. Write concise JSONL log events for decisions, commands, blockers, and
   produced artifacts.
3. Append TODO issue events for discovered actionable gaps.
4. Append a maturity reflection event after substantial changes or failed gates.
5. Re-run relevant validation gates before closing the task.
