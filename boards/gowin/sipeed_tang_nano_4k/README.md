# Sipeed Tang Nano 4K

Template board target (sipeed_tang_nano_4k, gowin_gw1ns).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: small
- DSP availability: low
- BRAM availability: none
- external memory availability: none
- useful interfaces: field_arithmetic, stream_infra, board_debug
- safe for beginner: true

## Board risk notes

- high-speed IO class: none
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
