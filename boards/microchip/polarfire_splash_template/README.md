# Microchip PolarFire Splash Template

Template board target (polarfire_splash_template, microchip_polarfire).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: large
- DSP availability: high
- BRAM availability: small_bram
- external memory availability: small_bram
- useful interfaces: field_arithmetic, plonk, stream_infra, audio
- safe for beginner: false

## Board risk notes

- high-speed IO class: pcie
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
