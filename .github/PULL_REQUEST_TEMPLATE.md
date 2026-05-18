## Summary

- What changed:
- Why:
- User-visible impact:

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo run -p af-cli --bin af -- self check --json`
- [ ] `cargo run -p af-cli --bin af -- registry check --json`

## Checklist

- [ ] Public docs match implemented CLI behavior.
- [ ] Generated files are either excluded or reproducible from documented commands.
- [ ] No private notes, local paths, credentials, or scratch artifacts are tracked.
- [ ] Claims avoid unsupported timing, CDC/RDC, security, vendor, or board signoff.
- [ ] License/SPDX implications were checked for new files.
