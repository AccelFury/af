---
name: Feature request
about: Propose a new af capability, CLI surface, evidence row, or contract change
labels: enhancement
---

## What is missing

<!-- One sentence: the gap or friction. -->

## Concrete scenario

<!-- A workflow you would run after this lands. Quote the commands. -->

```bash
```

## Where it would live

- New CLI subcommand under `af <namespace>`? (then `.claude/skills/af-add-subcommand` applies)
- New evidence row in `ReusableCoreMaturity`? (then `.claude/skills/af-add-evidence-row` applies, and it is breaking for `af core verify` consumers)
- New backend adapter under `crates/af-backend-*`?
- New manifest field? (touches `crates/af-manifest`, `schemas/af-core.schema.json`, `docs/manifest-reference.md`)
- New registry surface? (touches `registries/`, `schemas/cores.registry.schema.json`)
- Documentation only?

## Why now

<!-- What does this unblock? Is there a workaround? Any deadline? -->

## Out of scope / non-goals

<!-- What this feature is NOT about. Helps reviewers avoid scope creep. -->

## Notes

<!-- Optional: prior art, related issues, manifest snippet, output mock-up. -->
