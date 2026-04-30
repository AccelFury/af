# Contributing

Contributors must:

- follow repository structure and naming
- keep core RTL vendor-agnostic
- add/update docs for new IPs
- run checks before PR:
  - `cargo run -p af-cli --bin af -- vectors generate`
  - `cargo run -p af-cli --bin af -- core check cores/af-mod-add`
  - `cargo run -p af-cli --bin af -- registry check`
  - `cargo clippy --locked --workspace --all-targets -- -D warnings`
  - `cargo test --locked --workspace --no-fail-fast`
  - `deno task audit:repo`

Please keep code without placeholder language.
