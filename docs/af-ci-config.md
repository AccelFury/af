# af-ci.toml: схема и правила

Конфигурационный файл `af-ci.toml` является источником истины для рендера CI.

## Ключевые секции

- `[project]` — имя проекта, `hdl` и провайдер (`github`).
- `[paths]` — каталоги для сканирования (`rtl`, `tb`, `sim`, `formal`,
  `boards`).
- `[core]` — top-модуль и гло́бы исходников.
- `[simulation]` — профиль симуляции, команда, `pass_pattern`/`fail_pattern`.
- `[yosys]` — JSON синтез (`elab_and_synth_json`).
- `[artifacts]` — путь артефактов, `generate_sha256sums`, `store_tool_versions`.
- `[policy]` — безопасность и строгие правила `policy`:
  - `no_vendor_tools_in_public_ci`
  - `artifact_allowlist_only`
  - `no_unknown_script_execution`
- `[[boards]]` — список board profiles для optional P&R.

## Ожидаемые значения

- `af-ci.toml` должен быть валидным TOML.
- Для каждого board-профиля:
  - `family` в пределах поддерживаемых `gowin|ice40|ecp5`,
  - `top` обязателен,
  - `device` обязателен,
  - `constraints` указывает существующий файл,
  - `pack_device` обязателен для `ice40/ecp5`,
  - `nextpnr_family` и `pack_device` обязателены для `gowin`.

## Минимальная корректность

`af ci validate` отмечает как blocking ошибки:

- отсутствующие обязательные пути,
- core top не найден в RTL,
- invalid/отсутствующий constraints,
- нарушенный artifact allowlist,
- vendor tool / wildcard / secrets-policy violations.

## Чтение отчётов

JSON отчёты команд `init/doctor/validate` всегда содержат:

- `schema_version`
- `status`
- `detected` (detected profile/cores/top/candidates)
- `blocking_errors`
- `warnings`
- `problem_classes` (стабильные machine-readable классы проблем для
  doctor/validate)
- `artifact_contract` (список разрешённых путей при включённом allowlist)
