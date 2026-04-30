# Microchip IGLOO2 Template

Template board target (igloo2_template, microchip_igloo2).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: small
- DSP availability: low
- BRAM availability: none
- external memory availability: none
- useful interfaces: field_arithmetic, board_debug, stream_infra
- safe for beginner: true

## Board risk notes

- high-speed IO class: none
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
