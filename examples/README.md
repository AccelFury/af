# Examples

These examples are small reference surfaces for `af` workflows. They are not
timing, CDC/RDC, vendor, board, safety, or security signoff evidence.

| Example | Purpose | Green command | Known limitation |
| --- | --- | --- | --- |
| `simple-counter` | Minimal simple portable Verilog core | `af core check examples/simple-counter --json` | Basic structure example only. |
| `af-reset-sync` | Reset/CDC helper core | `af core sim examples/af-reset-sync --backend icarus --json` | Formal/vendor MTBF evidence is not populated. |
| `standards-ready-core` | Standards evidence layout | `af core standards check examples/standards-ready-core --strict --json` | Not safety/security certified. |
| `vendor-aware-skeleton` | Complex vendor-aware scaffold | `af core check examples/vendor-aware-skeleton --json` | Vendor backends are planned, not implemented. |
| `af-pdm-rx` | CI-enabled manifest-first smoke core | `af core report examples/af-pdm-rx --json` | No audio-quality or PDM-to-PCM claim. |

Recommended smoke:

```bash
af manifest validate examples/af-pdm-rx/af-core.toml --json
af core check examples/simple-counter --json
af core sim examples/af-reset-sync --backend icarus --json
af core standards check examples/standards-ready-core --strict --json
af core check examples/vendor-aware-skeleton --json
af core report examples/af-pdm-rx --json
```

Reports are written under the selected `--build-root` and can be attached to
issues or release evidence.
