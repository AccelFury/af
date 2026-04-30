# Board Targets

Tracked by template registry and generated in `boards/`:

- Sipeed Tang: `sipeed_tang_nano_1k`, `sipeed_tang_nano_4k`,
  `sipeed_tang_nano_9k`, `sipeed_tang_nano_20k`, `sipeed_tang_primer_20k_core`,
  `sipeed_tang_primer_20k_dock`, `sipeed_tang_primer_20k_lite`,
  `sipeed_tang_primer_25k`, `sipeed_tang_mega_138k`, `sipeed_tang_mega_138k_pro`
- Lattice iCE40: `icebreaker_ice40up5k`, `tinyfpga_bx_ice40lp8k`,
  `icestick_ice40hx1k`
- Lattice ECP5: `ulx3s_ecp5`, `orangecrab_ecp5`, `colorlight_5a_75b_ecp5`
- Intel: `terasic_de0_nano_cyclone4`, `terasic_de10_lite_max10`,
  `terasic_de10_nano_cyclonev_soc`, `qmtech_cyclone4_core`
- Xilinx: `digilent_basys3_artix7`, `digilent_arty_a7`, `digilent_cmod_a7`,
  `qmtech_artix7`, `ebaz4205_zynq7010`
- Efinix: `trion_t8`, `trion_t20`, `titanium_ti60`
- Microchip: `polarfire_splash_template`, `igloo2_template`

Compatibility templates:

- Legacy naming examples: `tang_nano_20k`, `tang_primer_20k`, `tang_mega_138k`,
  `cyclone4`.

For each target the repository keeps:

- board-level top in `boards/<vendor>/<board>/top/`
- constraint placeholders in `boards/<vendor>/<board>/constraints/`
- vendor build skeleton in `boards/<vendor>/<board>/<vendor>/build.tcl` (or
  flow-equivalent)
- `README.md` with explicit draft pinout warning
- `bringup.md` with bring-up checklist
- `board.status.json` with measurable statuses
