# Known Limitations

- `af` does not perform timing closure.
- `af` does not perform CDC/RDC signoff.
- RTL inspection is manifest-first and shallow.
- Vendor production flows are optional and not part of default CI.
- LiteX output is a reference skeleton, not a finished SoC integration.
- `af-pdm-rx` captures raw PDM bit groups only and does not generate PCM audio.
