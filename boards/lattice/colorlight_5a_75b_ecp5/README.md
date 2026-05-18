# ColorLight 5A-75B ECP5

Template board target (colorlight_5a_75b_ecp5, lattice_ecp5).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: large
- DSP availability: medium
- BRAM availability: small_bram
- external memory availability: small_bram
- useful interfaces: field_arithmetic, image_video, stream_infra
- safe for beginner: false

## Board risk notes

- high-speed IO class: serdes
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
