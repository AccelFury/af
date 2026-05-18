# Board bring-up checklist

- power and voltage domains verified against schematic
- LED heartbeat enabled and indicates simulation completion
- self-test ROM path checks are implemented
- button-triggered test path documented and safe by default
- UART/SPI debug remains optional and disabled by default
- verify all pins and voltage levels before first flash
- enable external SDRAM/DDR only after timing and memory map review
- do not enable external high-speed interfaces before signal integrity sign-off
