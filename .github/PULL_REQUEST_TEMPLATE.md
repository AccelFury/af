## Summary

- What changed:
- Why:
- User-visible impact:

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `.claude/skills/af-cli-contract-guard/check.sh`
- [ ] `cargo run -p af-cli --bin af -- self check --json`
- [ ] `cargo run -p af-cli --bin af -- registry check --json`
- [ ] `make smoke` for release-candidate or production-contract changes
- [ ] Tests were added or updated for the changed behavior, or the PR explains
      why direct test coverage is not possible.

## Checklist

- [ ] Public docs match implemented CLI behavior.
- [ ] Production-supported CLI/JSON/error/exit-code contract changes are
      documented in `CHANGELOG.md`.
- [ ] Release artifacts or smoke reports have checksums when this is a
      release-candidate change.
- [ ] Generated files are either excluded or reproducible from documented
      commands.
- [ ] No private notes, local paths, credentials, or scratch artifacts are
      tracked.
- [ ] Claims avoid unsupported timing, CDC/RDC, security, vendor, or board
      signoff.
- [ ] License/SPDX implications were checked for new files.
