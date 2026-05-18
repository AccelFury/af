---
name: Bug report
about: Something in af (CLI, RTL inspector, reports, schemas, docs) behaves wrong
labels: bug
---

## Summary

<!-- One sentence: what command or workflow misbehaves. -->

## Reproduction

```bash
# The exact commands you ran.
# Use --json so the failure has structured fields.
```

## Expected vs actual

- **Expected:**
- **Actual:**
- Exit code:
- `code` / `message` / `hint` (if structured): 

## Environment

- `af --version`:
- `cargo --version`:
- OS / arch:
- Relevant tool versions (`af doctor --json` output excerpt):
- Repo commit SHA:

## Attachments

<!-- Paste the `--json` payload if you have one. It contains the structured
     fields the af-error-explainer subagent reads. -->

```json
```

## Notes

<!-- Anything else: did `af registry check` pass? Did `af self check --json`
     pass on this tree? Tried a smaller reproducer? -->
