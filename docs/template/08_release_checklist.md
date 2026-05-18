# Release Checklist

1. `cargo test --workspace` passes.
2. `deno test --allow-read` passes.
3. `cargo xtask vectors` passes.
4. `cargo xtask sim-verilator` passes.
5. `cargo xtask lint-rtl` passes.
6. `deno task audit:repo` passes.
7. SPDX headers are present in all relevant source files.
8. README updated for the current IP revision.
9. Documentation updated for interface and behavior.
10. `ip.manifest.json` updated for each change.
11. Resource report attached or clearly marked as not yet synthesized.
12. Timing report attached or clearly marked as not yet closed.
13. Board targets tested or marked untested.
14. Commercial licensing files present (`COMMERCIAL.md`, `CLA.md`).
15. `CHANGELOG.md` updated.
16. Release tag created for the release commit.
