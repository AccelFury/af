# AF CI: политика безопасности

## Почему vendor tools не включаются по умолчанию

Open-source CI в `af` намеренно не запускает закрытые vendor-инструменты без
явного opt-in:

- минимизируется риск утечки лицензий,
- уменьшается размер и стоимость инфраструктуры пайплайна,
- соблюдается требование «не подменять доказательную базу» закрытым ПО.

## Правила, которые проверяет `af ci doctor / validate`

1. Нет `vendor tool launch` в публичном CI без policy-override.
2. Нет `| sh`/`|bash` и неизвестных shell-chain вызовов.
3. Нет wildcard upload вроде `path: .` или `path: ./`.
4. Нет загрузки секретов/секретных файлов (`.env`, `*.pem`, `secret`, `password`
   и т.п.).
5. Все artifact upload paths лежат в allowlist.
6. Без существующего P&R constraints не создаются board-artifacts.
7. Workflow содержит `pull_request`, `workflow_dispatch` и
   `permissions: contents: read`.
8. Все workflow `run` steps содержат `set -euo pipefail`.
9. `synth_core` содержит Yosys `hierarchy -check` и `write_json`.
10. Artifact contract содержит `tool-versions.txt`, Yosys JSON и `SHA256SUMS`.

В JSON `af ci doctor --json` и `af ci validate --json` эти проверки также
сводятся в стабильное поле `problem_classes`, например:

- `workflow_trigger_missing`
- `workflow_permissions_missing`
- `workflow_shell_safety_missing`
- `yosys_hierarchy_check_missing`
- `synth_json_missing`
- `tool_versions_missing`
- `sha256_missing`
- `artifact_upload_unsafe`
- `artifact_allowlist_violation`
- `vendor_tool_policy_violation`
- `unsafe_shell_pipe`
- `secret_artifact_policy_violation`

## Какие артефакты считаются безопасными по умолчанию

- `artifacts/openfpga-ci/logs/*.log`
- `artifacts/openfpga-ci/synth/*.json`
- `artifacts/openfpga-ci/pnr/*.json`
- `artifacts/openfpga-ci/reports/*.json`
- `artifacts/openfpga-ci/SHA256SUMS`
- `artifacts/openfpga-ci/logs/tool-versions.txt`

## Безопасная работа с зависимостями

- `af ci run-local` без `--dry-run` проверяет только базовые профили (`sim`,
  `synth`, `doctor`); для `iverilog_make` симуляции он требует `make`,
  `iverilog` и `vvp`, а для `synth` требует `yosys`.
- Информация о доступности tools/OSS CAD Suite фиксируется в отчётах и может
  использоваться при выборе инфраструктуры.
