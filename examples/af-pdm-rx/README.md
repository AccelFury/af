# af-pdm-rx

`af-pdm-rx` is the first AccelFury MVP example core.

Boundary:

- captures a raw PDM bitstream bit;
- emits a simple `sample_valid` pulse and `sample_bit`;
- forwards `clk` as `pdm_clk` for simple board experiments;
- explicitly does not implement PDM-to-PCM conversion.

Useful commands:

```bash
cargo run -p af-cli --bin af -- core check examples/af-pdm-rx
cargo run -p af-cli --bin af -- core lint examples/af-pdm-rx --backend verilator
cargo run -p af-cli --bin af -- wrapper generate examples/af-pdm-rx --target fusesoc
```
