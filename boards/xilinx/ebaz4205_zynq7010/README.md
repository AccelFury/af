# EBAZ4205 Zynq-7010

Template board target (ebaz4205_zynq7010, xilinx_zynq7000).

Draft only. Verify every pin against the official schematic and board revision
before programming hardware.

## Capabilities

- logic size class: medium
- DSP availability: high
- BRAM availability: external_sdram
- external memory availability: external_sdram
- useful interfaces: field_arithmetic, stream_infra, plonk, audio
- safe for beginner: false

## Board risk notes

- high-speed IO class: pcie
- clock domains and reset trees must be validated in this board wrapper.
- do not enable external high-speed interfaces before board-level signal
  integrity and voltage checks.
- voltage/ESD constraints must be confirmed from the vendor schematic.
