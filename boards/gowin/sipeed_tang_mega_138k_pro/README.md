# Sipeed Tang Mega 138K Pro

Template board target (sipeed_tang_mega_138k_pro, gowin_gw5a).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: large
- DSP availability: high
- BRAM availability: external_sdram
- external memory availability: external_sdram
- useful interfaces: field_arithmetic, ntt_fft, plonk, stream_infra
- safe for beginner: false

## Board risk notes

- high-speed IO class: lvds
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
