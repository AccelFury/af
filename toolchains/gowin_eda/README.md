# Gowin EDA toolchain template

Minimal skeleton for production flow:

- run project script from board folder (e.g.
  `boards/<vendor>/<board>/gowin/build.tcl`)
- collect reports in `reports/resources`
- install `gw_sh` and `programmer_cli` manually according to
  `docs/vendor-tooling.md`

`build.tcl` here is intentionally minimal and should be replaced with audited
scripts.
