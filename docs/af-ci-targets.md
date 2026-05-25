# CI Targets и board-профили

## Добавление платы

Добавить target можно через:

```bash
af ci add-board \
  --name tang_primer_20k_loopback \
  --family gowin \
  --top top_af_pdm_rx_loopback \
  --device GW2A-LV18PG256C8/I7 \
  --nextpnr-family GW2A-18C \
  --pack-device GW2A-18C \
  --constraints boards/sipeed_tang_primer_20k_dock/tang_primer_20k_dock.cst
```

Если обязательные поля отсутствуют, команда вернёт ошибку.

### Поддерживаемые board профили (минимальный релиз)

- **Gowin**: `family=gowin`, `top`, `device`, `nextpnr_family`, `pack_device`,
  `constraints`.
- **iCE40**: `family=ice40`, `top`, `device`, `pack_device`, `constraints`.
- **ECP5**: `family=ecp5`, `top`, `device`, `pack_device`, `constraints`.

`P&R` job генерируется только если профиль полный и constraints-файл существует.

## Добавление simulator profile

В первом релизе поддерживаются:

- `Verilog-2001 core` (без симуляции, если нет команд).
- `iverilog_make` (через `sim/Makefile` и `test` target).
- `sby_formal` (при наличии `.sby`).
- `generic_yosys` (BOM/JSON synthesis).

При явном `--sim "..."` в `af ci init`, симуляция всегда будет добавлена.

## Чтение top candidates

Сканер собирает имена модулей из `.v/.sv` и требует явный `--top`, если найдено
более одной кандидатной top-конфига.
