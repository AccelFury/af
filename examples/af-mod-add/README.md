# AccelFury Modular Add Core

`af-mod-add` is the template finite-field modular addition IP imported from
`core-template` snapshot `804bf1e`.

The core is kept as a normal AccelFury core package:

- `af-core.toml` is the canonical `af` manifest.
- `ip.manifest.legacy.json` preserves the source template manifest shape.
- `rtl/`, `tb/`, and `vectors/` are relative to this core directory.

RTL and gateware files in this package keep their original `CERN-OHL-S-2.0`
licensing. Rust and TypeScript tooling imported from `core-template` keeps
`AGPL-3.0-or-later` licensing.
