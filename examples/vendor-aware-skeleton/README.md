# vendor-aware-skeleton

Generated complex vendor-aware scaffold for projects that need a portable RTL
contract plus planned vendor RAM/DSP/clocking backends.

Useful checks:

```bash
af manifest validate examples/vendor-aware-skeleton/af-core.toml --json
af core check examples/vendor-aware-skeleton --json
af signoff plan examples/vendor-aware-skeleton --class complex-vendor-aware --json
```

Known limitations:

- vendor RAM/FIFO/DSP/clocking backends are planned, not implemented;
- no backend equivalence report exists;
- no timing closure, resource utilization, CDC/RDC, or hardware evidence exists.
