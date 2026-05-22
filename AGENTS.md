# AccelFury af Agent Rules

## Mandatory Completion Rule

- После любого хода, где агент выполнял работу (изменения файлов, анализ, запуск команд, тесты), проверить наличие незавершённых действий.
- Если есть хотя бы один незакрытый пункт (`pending` TODO, unresolved next step, открытый follow-up вопрос, отложенное действие), обязательно вызвать `$finish-all-tasks`.
- Выполнять цикл до состояния: `0 actionable items`, либо явный блокер с причиной.
- Не завершать ответ обязательными "следующими шагами", если агент может выполнить их сам в текущем контексте.
- Для блокеров указывать точную причину и что требуется для разблокировки.

## Active Source Of Truth

- `crates/**` owns behavior. Docs, skills, agents, reports, and mirrors follow crate behavior.
- `docs/cli-reference.md` owns CLI examples, flags, JSON contract prose, and exit-code documentation.
- `docs/agent-workflow.md`, `.claude/agents/**`, `.claude/skills/**`, and `CLAUDE.md` own active Claude guidance.
- Project `skills/af-*` owns the golden standard for installable Codex `af-*` skills.
- Installed Codex `~/.codex/skills/af-*` entries are convenience mirrors of `skills/af-*`, not a second source of truth.
- Ignored private workspaces and archived prep material are not authoritative for current `af` behavior.

## Validation

- Use `--json` for automation-facing `af` commands.
- Run `bash scripts/check-af-skills.sh` after changes to `af-*` skills, agents, `CLAUDE.md`, this file, project `skills/af-*`, or installed Codex `af-*` mirrors.
- Run `scripts/install-af-codex-skills.sh` to install project `skills/af-*` on another machine or refresh the local Codex mirror.
- Run `.claude/skills/af-cli-contract-guard/check.sh` before treating CLI, JSON, error-code, manifest, registry, or schema changes as safe.
