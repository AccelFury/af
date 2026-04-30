# Sipeed Tang Primer 25K

Template board target (sipeed_tang_primer_25k, gowin_gw5ast).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: large
- DSP availability: high
- BRAM availability: external_sram
- external memory availability: external_sram
- useful interfaces: ntt_fft, plonk, ai_tinyml, signal_processing
- safe for beginner: false

## Board risk notes

- high-speed IO class: lvds
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
