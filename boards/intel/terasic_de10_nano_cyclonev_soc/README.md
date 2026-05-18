# Terasic DE10-Nano Cyclone V SoC

Template board target (terasic_de10_nano_cyclonev_soc, intel_cyclonev_soc).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: large
- DSP availability: high
- BRAM availability: external_sdram
- external memory availability: external_sdram
- useful interfaces: field_arithmetic, plonk, stream_infra, audio
- safe for beginner: false

## Board risk notes

- high-speed IO class: serdes
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
