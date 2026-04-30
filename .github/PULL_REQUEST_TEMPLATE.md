## Summary

- What changed
- Why changed

## Validation

- [ ] cargo run -p af-cli --bin af -- vectors generate
- [ ] cargo run -p af-cli --bin af -- core lint cores/af-mod-add --backend verilator
- [ ] cargo run -p af-cli --bin af -- core sim examples/af-pdm-rx --backend verilator
- [ ] deno task audit:repo
- [ ] cargo test --locked --workspace --no-fail-fast
