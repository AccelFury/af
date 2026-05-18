# AF CI: Быстрый справочник

`af ci` формирует и проверяет CI-конфигурации для HDL/IP проектов на открытом стеке.
Первый релиз ориентирован на:

- Verilog-2001/2005 ядра,
- `iverilog + make` симуляцию (если есть `sim/Makefile`),
- Yosys JSON синтез,
- optional formal через `.sby`,
- optional board-level P&R (Open-Source nextpnr flow, только при наличии полного профиля).

## Что CI проверяет

- Yosys hierarchy/check и JSON-артефакт синтеза.
- Базовую симуляцию проекта, если команда симуляции задана.
- Формальную верификацию, если есть `.sby`.
- Сбор и публикацию артефактов (журналы, JSON, `SHA256SUMS`).

## Что CI не доказывает

- `CI` не доказывает vendor-tool implementation, timing signoff и CDC signoff.
- `CI` не заменяет in-field review и не является гарантом `hardware-ready`.
- Наличие артефактов означает факт прохождения шагов пайплайна, а не корректность изделия в производстве.

## Команды

```bash
af ci init --project <name> --hdl <verilog-2001|verilog-2005> --rtl <path> --top <module> [--sim <cmd>]
af ci render --config af-ci.toml --output .github/workflows/hdl-ci.yml [--dry-run]
af ci doctor --repo .
af ci improve --repo . [--allow-rewrite] [--dry-run]
af ci add-board --repo . --name <name> --family <gowin|ice40|ecp5> --top <module> --device <dev> --constraints <path> [--package <pkg>]
af ci validate --repo . [--config af-ci.toml]
af ci run-local --repo . --profile sim|synth|doctor [--dry-run]
```

`af ci run-local --profile sim` checks that `make` is available, and for the
Icarus Makefile profile it also requires both `iverilog` and `vvp` in `PATH`.
`af ci run-local --profile synth` requires `yosys` in `PATH`.

Рекомендуемый цикл:

1. `af ci init ...` (или ручное редактирование `af-ci.toml`);
2. `af ci render ...`;
3. `af ci doctor --repo .` и `af ci validate`;
4. `git add docs/ci.md .github/workflows/hdl-ci.yml .github/PULL_REQUEST_TEMPLATE.md af-ci.toml`.

`--json` output для `doctor` и `validate` содержит `problem_classes`: стабильные
коды вроде `workflow_trigger_missing`, `synth_json_missing`,
`artifact_upload_unsafe`, `vendor_tool_policy_violation`. Их можно использовать
в pre-merge bots без парсинга человекочитаемых строк.

## Артефакты

- `artifacts/openfpga-ci/logs/tool-versions.txt`
- `artifacts/openfpga-ci/logs/*.log`
- `artifacts/openfpga-ci/synth/*.json`
- `artifacts/openfpga-ci/pnr/*.json`
- `artifacts/openfpga-ci/reports/*.json`
- `artifacts/openfpga-ci/SHA256SUMS`
- `artifacts/openfpga-ci/reports/af-ci-init-report.json`

Дополнительно смотрите:
- `docs/af-ci-config.md`
- `docs/af-ci-targets.md`
- `docs/af-ci-security.md`
- `docs/af-ci-examples.md`
