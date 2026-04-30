# Release Process

Before release:

- run `cargo fmt --all -- --check`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `cargo test --workspace`;
- run CLI smoke on `examples/af-pdm-rx`;
- verify generated FuseSoC and LiteX artifacts;
- confirm reports contain versions, commands, artifacts, warnings, and
  limitations;
- update changelog and known limitations.
