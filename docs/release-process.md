# Release Process

## Alpha readiness

Before marking the repository alpha-ready:

- confirm the supported alpha command set is documented in
  `docs/cli-reference.md`;
- keep staged claims explicit: timing closure, CDC/RDC signoff, vendor
  production bitstreams, and hardware programming are not alpha guarantees;
- run `cargo fmt --all -- --check`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `cargo test --workspace`;
- run `af self check --json`;
- run CLI smoke on `examples/af-pdm-rx` for `doctor`, `manifest validate`,
  `core check`, `core report`, `core lint --backend native`,
  `wrapper generate`, and `ci generate`;
- run `af core sim examples/af-reset-sync --backend icarus --json` as the
  host-side simulation smoke;
- verify optional HDL backend absence returns structured unavailable or
  documented failure envelopes instead of panics;
- update changelog, roadmap, and known limitations in the same change.

Docker smoke is recommended for alpha readiness when the Docker runtime is
available, but missing host HDL tools are not a blocker if they report
structured `BackendUnavailable` state.

## Production release gate

Before marking `af` production-ready as a CLI/toolchain:

- confirm `docs/production-readiness.md`, `docs/cli-reference.md`,
  `docs/known-limitations.md`, README, and the PR template agree on supported
  and unsupported claims;
- run `cargo fmt --all -- --check`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `cargo test --workspace`;
- run `.claude/skills/af-cli-contract-guard/check.sh`;
- run host CLI smoke for `doctor`, `self check`, `manifest validate`,
  `core check`, `core report`, `core lint --backend native`,
  `wrapper generate`, and `ci generate`;
- run `make smoke` so Docker covers Verilator, Yosys, FuseSoC, LiteX skeleton,
  SMT solver visibility, package/report contracts, and migration dry-run;
- capture `SHA256SUMS` for release artifacts and smoke reports;
- require external CI evidence for release claims: workflow run URL, commit SHA,
  conclusion `success`, artifact bundle, and checksums;
- update `CHANGELOG.md` with every additive or breaking CLI, manifest, JSON,
  schema, exit-code, or error-code contract change.

Production-ready `af` does not promote timing closure, CDC/RDC signoff, vendor
bitstream production, board-ready, hardware-ready, or security-certification
claims. Those require separate evidence and must remain limitations otherwise.

## General release

Before release:

- run `cargo fmt --all -- --check`;
- run `cargo clippy --workspace --all-targets -- -D warnings`;
- run `cargo test --workspace`;
- run `af self check --json` and CLI smoke on `examples/af-pdm-rx`;
- verify generated FuseSoC and LiteX artifacts;
- confirm reports contain versions, commands, artifacts, warnings, and
  limitations;
- update changelog and known limitations.
