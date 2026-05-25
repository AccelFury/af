# Known Limitations

- Alpha readiness covers the manifest-first CLI workflow only. Commands outside
  `doctor`, `self check`, `manifest validate`, `core check`, `core lint`,
  `core sim`, `core report`, `core standards check`, `core standards export`,
  `wrapper generate`, and `ci generate` may still change before v1.0.
- `af` does not perform timing closure.
- `af` does not perform CDC/RDC signoff.
- RTL inspection is manifest-first and shallow.
- `af core standards check` performs deterministic, lightweight semantic
  validation for declared and conventional standards artifacts. It does not
  replace normative XML schema tooling, vendor signoff, legal review, safety
  assessment, or security certification. `--strict` requires selected external
  validators when checking matching artifacts, but it still does not constitute
  certification or vendor signoff.
- Vendor production flows are optional and not part of default CI.
- Bitstream production and hardware programming are staged flows, not alpha
  guarantees.
- Production-ready status for `af` means CLI/toolchain contract readiness only;
  it does not imply timing closure, CDC/RDC signoff, vendor bitstreams, or
  hardware-ready status without the evidence listed in
  `docs/production-readiness.md`.
- LiteX output is a reference skeleton, not a finished SoC integration.
- `af-pdm-rx` captures raw PDM bit groups only and does not generate PCM audio.
- Hard PHY blocks (DDR PHY, PCIe hard IP, MIPI D-PHY, high-speed SerDes such as
  GTX/GTY/GTH/GTP) are out of portable-RTL scope. The RTL inspector emits the
  distinct code `AF_PORTABLE_HARD_PHY_BLOCK` (separate from the generic
  `AF_PORTABLE_VENDOR_OR_CLOCK_MARKER`) so authors can immediately reclassify
  the core as `complex-vendor-aware` at `portability_level = U3` or `U4` and
  move the instantiation into a vendor wrapper.
