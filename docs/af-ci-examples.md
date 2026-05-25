# AF CI: примеры и эксплуатация

## Минимальный core-only init

```bash
af ci init \
  --project af_mod_add \
  --hdl verilog-2001 \
  --rtl rtl \
  --top af_mod_add \
  --provider github
```

Результат: workflow, `docs/ci.md`, `af-ci.toml`, PR template и
`artifacts/.../af-ci-init-report.json`.

## С существующим `sim/Makefile`

```bash
af ci init \
  --project af_mod_add \
  --hdl verilog-2001 \
  --rtl rtl \
  --top af_mod_add \
  --sim "cd sim && make test" \
  --provider github
```

CI добавит реальную симуляцию (без `--fake` фейковых команд).

## Добавление board профилей

```bash
af ci add-board \
  --family ecp5 \
  --name ulx3s \
  --top top_ulx3s \
  --device 85k \
  --package CABGA381 \
  --constraints boards/ulx3s/ulx3s.lpf
```

```bash
af ci add-board \
  --family gowin \
  --name tang_primer_20k_loopback \
  --top top_af_pdm_rx_loopback \
  --device GW2A-LV18PG256C8/I7 \
  --nextpnr-family GW2A-18C \
  --pack-device GW2A-18C \
  --constraints boards/sipeed_tang_primer_20k_dock/tang_primer_20k_dock.cst
```

## Чтение артефактов CI

- Логи: `artifacts/openfpga-ci/logs/*.log`
- Версии инструментов: `artifacts/openfpga-ci/logs/tool-versions.txt`
- JSON синтеза: `artifacts/openfpga-ci/synth/*.json`
- JSON P&R: `artifacts/openfpga-ci/pnr/*.json`
- Агрегированные отчёты: `artifacts/openfpga-ci/reports/*.json`
- Контроль целостности: `artifacts/openfpga-ci/SHA256SUMS`

## Как использовать `af ci doctor`

```bash
af ci doctor --repo .
```

Команда возвращает:

- `pass` / `warning` / `fail` в JSON и код завершения,
- список blocking errors и warnings,
- перечень next_actions.

## OSS CAD Suite / toolchain

- `af ci` использует локально установленные утилиты (`make`, `yosys`,
  `iverilog`, `vvp`, `sby`, `xmllint`, `fusesoc`) и SMT solvers для formal flows
  (`boolector`, `z3`, `yices-smt2`, `bitwuzla`, `cvc5`); Edalize проверяется как
  Python module для package/export flows,
- для обновления версий зависимостей используйте локальную инфраструктуру
  установки toolchain в вашем окружении,
- vendor-tool flows (например, прямой `vivado/diamond`) в первом релизе не
  включаются автоматически.
