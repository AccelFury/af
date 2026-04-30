# Overview

`af_mod_add` is the first concrete IP in this template and demonstrates:

- fixed-latency arithmetic core
- ready/valid streaming input/output
- deterministic vector-driven verification
- multi-board integration layout
- Verilog-2001, Verilog-2005, and SystemVerilog source compatibility contract

The template itself is intended to be copied and specialized into other
ZK-friendly IPs such as:

- `af_mod_sub`
- `af_mod_mul_goldilocks`
- `af_ntt_butterfly`
- `af_ntt_core`
- `af_poseidon_t3`
- `af_mimc_sponge`
- `af_merkle_path_verify`
- `af_plonk_gate_eval`
- `af_plonk_perm_numden`
- `af_grand_product`
