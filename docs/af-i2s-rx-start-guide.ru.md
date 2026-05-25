# Старт разработки `af-i2s-rx` с помощью `af`

Это руководство описывает практический старт нового FPGA/IP-проекта `af-i2s-rx`
в текущем монорепозитории AccelFury `af`. Цель первого этапа - получить
воспроизводимый scaffold, корректный `af-core.toml`, начальный RTL,
smoke-testbench и набор локальных проверок, которые можно запускать до появления
полноценной board-интеграции.

## Рабочая модель

`af-i2s-rx` следует начинать как переиспользуемый IP core, а не как
board-specific проект. В терминах текущего репозитория это означает:

- источник истины для IP - `af-core.toml`;
- RTL живет рядом с манифестом в `rtl/`;
- тестовые окружения живут в `tb/`;
- board-top, constraints и vendor scripts добавляются позже через существующие
  board/toolchain поверхности;
- сгенерированные отчеты и wrappers пишутся в `.af-build/` и не должны
  редактироваться вручную.

Рекомендуемое место для первой итерации - `examples/af-i2s-rx`. Когда интерфейс,
тесты и ограничения стабилизируются, core можно продвигать в `cores/af-i2s-rx`.

Во всех командах ниже используется запуск `af` из исходников:

```bash
cargo run -p af-cli --bin af --
```

Если локальный бинарь `af` уже установлен в `PATH`, заменяйте этот префикс на
`af`.

## Предварительные проверки

Из корня репозитория выполните базовую диагностику:

```bash
cargo run -p af-cli --bin af -- doctor --json
cargo run -p af-cli --bin af -- registry check
cargo run -p af-cli --bin af -- board list
```

`doctor` проверяет видимость открытых backend-инструментов. Отсутствие
Verilator, Yosys, FuseSoC или LiteX не блокирует scaffold и manifest-проверки,
но команды lint/sim/build вернут структурированную ошибку backend unavailable.
Для полной open-source smoke-проверки в репозитории предусмотрен Docker flow:

```bash
make smoke
```

## Шаг 1. Зафиксировать границы MVP

Перед генерацией кода запишите короткий engineering brief. Для первой версии
`af-i2s-rx` границы должны быть узкими:

- принимает стандартный Philips I2S: `sck`, `ws/lrclk`, `sd`;
- захватывает PCM-сэмплы MSB-first с задержкой на один bit clock после фронта
  `ws`;
- поддерживает параметризуемую ширину сэмпла, например 16/24/32 бита;
- выдает один packed stereo frame через valid-ready поток;
- не выполняет DSP, ресемплинг, фильтрацию, gain, mute/declick, clock recovery
  или audio-quality оценку;
- явно описывает clocking assumption: либо входы I2S синхронизируются в `clk`,
  либо захват идет в домене `i2s_sck_i` с отдельным CDC на границе системы.

Для начального portable MVP проще держать публичный IP-интерфейс в домене `clk`
и синхронизировать внешние `i2s_*` сигналы внутри core. Тогда в
`known_limitations` нужно явно указать, что `clk` должен быть достаточно быстрее
`i2s_sck_i`, а board timing constraints обязаны подтвердить этот режим. Если
проект сразу целится в точный audio/board bring-up, лучше оформить `i2s_sck_i`
как отдельный clock domain и планировать CDC до system stream.

## Шаг 2. Сгенерировать scaffold

Создайте core через `af core new`. Это единая команда для новых базовых ядер; по
умолчанию она использует portable Verilog-2001 профиль `stream-ip` и создает
валидный `af_version = "0.2"` scaffold.

```bash
cargo run -p af-cli --bin af -- core new examples/af-i2s-rx \
  --name af-i2s-rx \
  --library audio
```

Ожидаемая структура:

```text
examples/af-i2s-rx/
  af-core.toml
  rtl/
    af_i2s_rx.v
```

Имя директории и package name остаются `af-i2s-rx`, а имя RTL-модуля становится
`af_i2s_rx`, потому что SystemVerilog identifier не должен содержать дефисы.

Сразу зафиксируйте baseline:

```bash
cargo run -p af-cli --bin af -- manifest validate examples/af-i2s-rx/af-core.toml
cargo run -p af-cli --bin af -- core check examples/af-i2s-rx
```

Если scaffold уже существует, команда откажется перезаписывать файлы. В этом
случае выберите новый путь для эксперимента или осознанно разберите старую
директорию вручную.

## Шаг 3. Заменить generic scaffold на I2S boundary

После генерации scaffold замените порты `enable/done` на I2S RX интерфейс и
valid-ready stream. Для первой версии удобно начать с packed stereo frame:

```verilog
module af_i2s_rx
#(
  parameter SAMPLE_BITS = 24,
  parameter FRAME_BITS  = 48
)
(
  input wire                   clk,
  input wire                   rst_n,
  input wire                   i2s_sck_i,
  input wire                   i2s_ws_i,
  input wire                   i2s_sd_i,
  output reg [FRAME_BITS-1:0]  sample_data_o,
  output reg                   sample_valid_o,
  input wire                   sample_ready_i
);
```

Минимальная семантика `sample_data_o` должна быть документирована сразу.
Например: `sample_data_o = {left_sample, right_sample}`, каждый sample -
two's-complement PCM, sign-extended или zero-padded согласно выбранному
требованию. Не оставляйте packing implicit: downstream wrappers и тесты будут
ориентироваться на манифест.

Для RTL первой итерации достаточно реализовать:

- двухтактную синхронизацию `i2s_sck_i`, `i2s_ws_i`, `i2s_sd_i` в домен `clk`,
  если выбран single-clock MVP;
- edge-detect для `i2s_sck_i`;
- определение нового left/right frame по изменению `i2s_ws_i`;
- пропуск первого bit clock после изменения `ws` для Philips I2S;
- shift register на `SAMPLE_BITS`;
- сбор left/right samples в `sample_data_o`;
- удержание `sample_valid_o` до `sample_ready_i`.

Не добавляйте в первый RTL audio-quality заявления. Core должен честно говорить:
он декодирует serial I2S frame в raw PCM frame, но не доказывает качество
звукового тракта.

## Шаг 4. Обновить `af-core.toml`

Манифест должен описывать фактический RTL. После замены портов приведите
`examples/af-i2s-rx/af-core.toml` примерно к такой форме:

```toml
af_version = "0.2"
name = "af-i2s-rx"
vendor = "accelfury"
library = "audio"
core = "af_i2s_rx"
version = "0.1.0"
known_limitations = [
  "Initial MVP captures Philips I2S frames only; left-justified, right-justified and TDM modes are out of scope.",
  "System-clock oversampling assumptions and board timing constraints must be validated per target.",
  "Core does not perform DSP, resampling or audio-quality validation."
]

[metadata]
display_name = "AccelFury I2S RX"
license = "Apache-2.0"
authors = ["AccelFury contributors"]
description = "I2S receiver core that converts serial Philips I2S input into a packed valid-ready PCM frame stream."

[rtl]
top = "af_i2s_rx"
language = "verilog-2001"
default_clock = "clk"
default_reset = "rst_n"

[sources]
files = ["rtl/af_i2s_rx.v"]
include_dirs = []

[[parameters]]
name = "SAMPLE_BITS"
kind = "integer"
default = "24"
allowed = ["16", "24", "32"]
description = "Bits captured per channel from the I2S serial data stream."

[[parameters]]
name = "FRAME_BITS"
kind = "integer"
default = "48"
allowed = ["32", "48", "64"]
description = "Packed stereo frame width: left sample followed by right sample."

[[clocks]]
name = "clk"
port = "clk"
frequency_hz = 27000000
description = "System clock used by the initial single-clock MVP."

[[resets]]
name = "rst_n"
port = "rst_n"
active = "low"
style = "async"
clock_domain = "clk"

[[ports]]
name = "clk"
direction = "input"
width = 1
kind = "clock"
clock_domain = "clk"

[[ports]]
name = "rst_n"
direction = "input"
width = 1
kind = "reset"
active = "low"
reset_style = "async"
clock_domain = "clk"

[[ports]]
name = "i2s_sck_i"
direction = "input"
width = 1
kind = "clock_like_data"
interface = "i2s_rx"

[[ports]]
name = "i2s_ws_i"
direction = "input"
width = 1
kind = "word_select"
interface = "i2s_rx"

[[ports]]
name = "i2s_sd_i"
direction = "input"
width = 1
kind = "serial_data"
interface = "i2s_rx"

[[ports]]
name = "sample_data_o"
direction = "output"
width = "FRAME_BITS"
kind = "data"
interface = "pcm_stream"
clock_domain = "clk"

[[ports]]
name = "sample_valid_o"
direction = "output"
width = 1
kind = "valid"
interface = "pcm_stream"
clock_domain = "clk"

[[ports]]
name = "sample_ready_i"
direction = "input"
width = 1
kind = "ready"
interface = "pcm_stream"
clock_domain = "clk"

[[interfaces]]
name = "i2s_rx"
kind = "i2s_philips_rx"
clock = "i2s_sck_i"

[[stream_interfaces]]
name = "pcm_stream"
kind = "valid_ready"
clock_domain = "clk"
data = "sample_data_o"
valid = "sample_valid_o"
ready = "sample_ready_i"
data_width = "FRAME_BITS"
payload_semantics = "packed_stereo_pcm_left_then_right"

[[testbenches]]
name = "verilator_smoke"
backend = "verilator"
top = "tb_af_i2s_rx"
sources = ["tb/tb_af_i2s_rx.sv"]
rtl_sources = ["rtl/af_i2s_rx.v"]
expected = "pass"

[formal]
name = "basic_stream_contract"
backend = "sby"
enabled = false
files = []

[backend_compatibility]
verilator = true
fusesoc = true
```

После правки манифеста снова запустите:

```bash
cargo run -p af-cli --bin af -- manifest validate examples/af-i2s-rx/af-core.toml
cargo run -p af-cli --bin af -- core check examples/af-i2s-rx
```

`core check` дополнительно проверит, что `rtl.top` действительно встречается в
исходниках как модуль.

## Шаг 5. Добавить smoke-testbench

Создайте `examples/af-i2s-rx/tb/tb_af_i2s_rx.sv`. Первый testbench должен быть
маленьким, но обязан ловить основные ошибки протокола:

- после reset `sample_valid_o == 0`;
- `ws` переключается до MSB, а первый `sck` после переключения пропускается;
- left и right samples собираются в ожидаемом порядке;
- `sample_valid_o` удерживается при `sample_ready_i == 0`;
- после `sample_ready_i == 1` core принимает следующий frame.

Минимальный сценарий:

1. Подать reset на несколько тактов `clk`.
2. Сгенерировать I2S frame с известными значениями, например left `0x123456`,
   right `0xABCDEF` при `SAMPLE_BITS = 24`.
3. Проверить packed output `{left, right}`.
4. Повторить frame с backpressure на `sample_ready_i`.

Для smoke-level проверки достаточно `$fatal` при несовпадении. Позже можно
добавить randomized frames, разные `SAMPLE_BITS`, assertions на valid-ready и
отдельный regression vector set.

## Шаг 6. Запустить локальные проверки core

Базовый цикл после каждой значимой правки:

```bash
cargo run -p af-cli --bin af -- manifest validate examples/af-i2s-rx/af-core.toml
cargo run -p af-cli --bin af -- core check examples/af-i2s-rx
cargo run -p af-cli --bin af -- core lint examples/af-i2s-rx --backend verilator
cargo run -p af-cli --bin af -- core sim examples/af-i2s-rx --backend verilator
cargo run -p af-cli --bin af -- wrapper generate examples/af-i2s-rx --target fusesoc
cargo run -p af-cli --bin af -- core report examples/af-i2s-rx
```

Если Verilator не установлен, первые две команды все равно должны проходить. Для
машинного разбора добавляйте `--json`.

## Шаг 7. Проверить board-направление без преждевременной привязки

Когда core проходит manifest/check/lint/sim, можно проверить доступные board
targets:

```bash
cargo run -p af-cli --bin af -- board list
cargo run -p af-cli --bin af -- wrapper generate examples/af-i2s-rx --target litex --board tang-nano-20k
cargo run -p af-cli --bin af -- build examples/af-i2s-rx --board tang-nano-20k --backend litex
```

На этом этапе board build не заменяет RTL verification. Он проверяет, что core
можно подать в существующий backend/wrapper path. Реальное подключение к
PMOD/audio codec/микрофону потребует отдельного board top, constraints,
pinout-документа и timing review.

## Шаг 8. Документировать ограничения и критерии готовности

Для `af-i2s-rx` особенно важно не смешивать декодирование протокола и качество
аудио. В `known_limitations`, README core и release notes держите явные границы:

- supported: Philips I2S RX, fixed sample widths, packed stereo PCM frame;
- unsupported in MVP: left-justified, right-justified, TDM, multi-lane,
  resampling, FIFO depth tuning, clock recovery, audio SNR/THD claims;
- board-dependent: max `i2s_sck_i`, setup/hold на входах, CDC policy,
  constraints, pin mapping, physical connector assumptions.

Первый этап можно считать завершенным, когда выполняются условия:

- `examples/af-i2s-rx/af-core.toml` валиден;
- `af core check examples/af-i2s-rx` проходит;
- smoke-testbench проверяет left/right frame ordering и backpressure;
- `af core lint` и `af core sim` проходят там, где доступен Verilator;
- FuseSoC wrapper генерируется;
- README core описывает ports, parameters, payload packing и limitations;
- ни один документ не заявляет timing closure или audio quality без
  соответствующего отчета.

## Типичные ошибки

- `AF_MANIFEST_INVALID`: манифест ссылается на несуществующий clock/reset/port,
  параметр ширины или путь. Исправьте `af-core.toml`, затем повторите
  `manifest validate`.
- `AF_TOP_MODULE_MISSING`: `rtl.top` не совпадает с именем модуля в RTL. Для
  `af-i2s-rx` ожидаемое имя модуля - `af_i2s_rx`.
- `AF_BACKEND_UNAVAILABLE`: backend-инструмент не найден в `PATH`. Используйте
  Docker smoke flow или установите нужный backend локально.
- `sample_valid_o` теряется при backpressure: valid-ready контракт нарушен.
  Output data и valid должны удерживаться, пока downstream не поднимет ready.
- I2S sample сдвинут на один бит: обычно это означает, что RTL не учел one-bit
  delay после переключения `ws` в Philips I2S.

## Рекомендуемый порядок дальнейшего развития

1. MVP RTL + smoke-testbench в `examples/af-i2s-rx`.
2. Расширенный testbench: widths 16/24/32, randomized frames, backpressure.
3. README core с interface contract и known limitations.
4. LiteX/FuseSoC wrapper generation check.
5. Board-specific top и constraints только после стабильного core contract.
6. Timing/report artifacts для выбранной платы.
7. Promotion из `examples/` в `cores/` после повторяемых проверок и понятного
   release checklist.
