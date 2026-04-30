# Contributing

Contributors must:

- follow repository structure and naming
- keep core RTL vendor-agnostic
- add/update docs for new IPs
- run checks before PR:
  - `cargo xtask vectors`
  - `cargo xtask lint-rtl`
  - `cargo xtask sim-verilator`
  - `deno task audit:repo`
  - `cargo xtask check-spdx`

Please keep code without placeholder language.
