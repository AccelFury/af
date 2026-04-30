# QMTech Cyclone 4 Core

Template board target (qmtech_cyclone4_core, intel_cyclone4).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: medium
- DSP availability: low
- BRAM availability: small_bram
- external memory availability: small_bram
- useful interfaces: field_arithmetic, board_debug, stream_infra
- safe for beginner: true

## Board risk notes

- high-speed IO class: none
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
