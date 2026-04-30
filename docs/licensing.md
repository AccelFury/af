# Licensing

The merged monorepo preserves source licensing instead of relicensing imported
surfaces.

- Existing Apache-licensed `af` Rust crates keep `Apache-2.0` via workspace package metadata.
- Imported Rust and TypeScript tooling from `core-template` keeps `AGPL-3.0-or-later`.
- RTL and gateware under imported core/template surfaces keep `CERN-OHL-S-2.0`.
- Imported documentation keeps `CC-BY-SA-4.0` where applicable.
- Full license texts are stored in `LICENSES/`.

Do not infer a single-project relicensing from file location alone. SPDX headers
and package metadata are the source of truth for individual files.
