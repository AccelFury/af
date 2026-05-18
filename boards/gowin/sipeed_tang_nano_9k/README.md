# Sipeed Tang Nano 9K

Template board target (sipeed_tang_nano_9k, gowin_gw2a).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: medium
- DSP availability: medium
- BRAM availability: small_bram
- external memory availability: small_bram
- useful interfaces: field_arithmetic, ntt_fft, dsp, stream_infra
- safe for beginner: true

## Board risk notes

- high-speed IO class: none
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
