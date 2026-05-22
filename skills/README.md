# AccelFury af Codex Skills

This directory is the project-local golden source for installable Codex
`af-*` skills.

## Contents

- `af-heal`: repair drift across `af` docs, governance, generated outputs,
  installed mirrors, and stale active guidance.
- `af-update`: synchronize canonical `af` surfaces into derived docs,
  generated outputs, and installed skill mirrors.
- `af-upgrade`: improve `af` functional maturity, reliability, usability,
  verification depth, and agent operability.

## Install

From the repository root:

```bash
scripts/install-af-codex-skills.sh
```

The script copies `skills/af-*` into `${CODEX_HOME:-$HOME/.codex}/skills`.
Installed skills are mirrors. Edit the project files first, then reinstall.

## Validate

```bash
bash scripts/check-af-skills.sh
```
