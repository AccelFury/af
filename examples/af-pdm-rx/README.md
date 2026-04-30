# af-pdm-rx

`af-pdm-rx` is the first AccelFury MVP example core.

Boundary:

- accepts a 1-bit raw PDM bitstream on `pdm_data_i`;
- generates a divided `pdm_clk_o`;
- groups raw PDM bits into `sample_word_o`;
- exposes a `sample_valid_o` / `sample_ready_i` valid-ready stream;
- explicitly does not implement PDM-to-PCM conversion.
- does not make audio-quality claims because no PCM audio is generated.

Useful commands:

```bash
cargo run -p af-cli --bin af -- core check examples/af-pdm-rx
cargo run -p af-cli --bin af -- core lint examples/af-pdm-rx --backend verilator
cargo run -p af-cli --bin af -- wrapper generate examples/af-pdm-rx --target fusesoc
cargo run -p af-cli --bin af -- wrapper generate examples/af-pdm-rx --target litex --board tang-nano-20k
```
