# Manifest Reference

`af-core.toml` v0.1/v0.2 describes one IP core.

If `[rtl].language` is omitted, the parser defaults to `verilog-2001`.

Required root fields:

- `af_version = "0.1"` or `af_version = "0.2"`
- `name`
- `vendor`
- `library`
- `core`
- `version`

Required tables:

```toml
[rtl]
top = "my_core"
language = "verilog-2001"
default_clock = "clk"
default_reset = "rst_n"

[sources]
files = ["rtl/my_core.v"]
include_dirs = []
```

Supported arrays:

- `[[parameters]]`: `name`, `value`, optional `description`
- `[[ports]]`: `name`, `direction`, optional `width`, `clock`, `reset`, `description`
- `[[clocks]]`: `name`, optional `frequency_hz`
- `[[resets]]`: `name`, optional `active`, `asynchronous`
- `[[interfaces]]`: `name`, `kind`, optional `clock`, `reset`
- `[[testbenches]]`: `name`, `top`, `sources`
- `[[vectors]]` in v0.2: `name`, `format`, `path`

Optional v0.2 fields:

```toml
category = "field_arithmetic"

[rtl.variants]
systemverilog = ["rtl/core.sv"]
verilog_2001 = ["rtl/core.v"]

[tooling]
rust = true
typescript_deno = true
python = false
cocotb = false
fusesoc_required = false
```

Optional metadata:

```toml
[metadata]
license = "Apache-2.0"
authors = ["Example"]
repository = "https://example.invalid/repo"
description = "Core description"
```

Validation rules:

- all manifest paths must be relative and must not contain `..`;
- port widths must be positive integers;
- port/interface clock and reset references must be declared;
- RTL language must be `systemverilog`, `verilog`, `verilog-2001`, or `vhdl`;
- `sources.files` must not be empty.
