# Microarchitecture

The core is a one-stage register with fixed latency 1 cycle:

- combinational modular addition computed from `i_a` + `i_b`
- register capture only when input handshake occurs (`i_valid && o_ready`)
- explicit output handshake backpressure via `i_ready`

Throughput is one accepted transaction per cycle when downstream is ready.
Because output is single-entry buffered, latency is predictable and stable for
validation workflows.
