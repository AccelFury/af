Generated vectors are stored under this directory:

- `af_mod_add_basic.json` with fixed edge-case vectors.
- `af_mod_add_random.json` with deterministic pseudo-random vectors.
- `af_mod_add_random.svh` for direct Verilator testbench include.

Generation is handled by:

```bash
cargo xtask vectors
```
