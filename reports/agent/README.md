# Agent Reports

LLM agents write compact session evidence here.

Use JSON Lines logs under:

```text
reports/agent/logs/YYYY-MM-DD/<run_id>.jsonl
```

Each line must follow the logging schema in
`docs/agent/agent-operating-protocol.yaml`.

Do not log secrets, credentials, private keys, tokens, serial numbers, or
customer data. Prefer concise command summaries and local artifact paths over
large pasted outputs.
