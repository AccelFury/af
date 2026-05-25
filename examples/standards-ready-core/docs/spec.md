<!-- SPDX-License-Identifier: Apache-2.0 -->

# standards-ready-core Standards Evidence Spec

## 1. Purpose

Demonstrate a complete standards evidence layout for a small portable
ready/valid FPGA IP core.

## 2. Target Users

FPGA developers evaluating the `af core standards` flow.

## 3. Use Cases

Use as a local reference for manifest, IP-XACT, SystemRDL, SPDX/HBOM, CI, and
commercial-baseline evidence.

## 4. Non-Goals

This core does not claim hardware validation, timing signoff, safety
certification, or security certification.

## 5. Portability Tier

U0 portable Verilog-2001 RTL, single clock domain, synchronous active-high
reset, no vendor primitives.

## 6. Interface

The core accepts one ready/valid input stream and emits one ready/valid output
stream. `cfg_enable` gates acceptance. `status_busy` reports backpressure.

## 7. Parameters

`DATA_WIDTH` controls stream data width. The example manifest pins it to 8.

## 8. Architecture

One output holding register stores the accepted word until downstream
`m_ready`.

## 9. Datapath / Control FSM

The control path is implicit: empty, hold, and transfer states are encoded by
`m_valid` and handshake signals.

## 10. Reset / Clock / CDC Behaviour

Single `clk` domain. `rst` is active-high synchronous. There is no CDC.

## 11. Protocol Semantics

Input transfer occurs when `s_valid && s_ready`. Output transfer occurs when
`m_valid && m_ready`.

## 12. Error / Status / Counter Behaviour

`status_busy` is asserted while an output word is waiting. `counter_words`
counts accepted input words modulo 256.

## 13. Timing and Latency

Latency is one cycle from accepted input to valid output.

## 14. Corner Cases

Backpressure holds `m_valid` and `m_data` stable. Disabled input keeps
`s_ready` low.

## 24. IP-XACT Packaging

See `ipxact/standards_ready_core.xml`.

## 25. Register Description

See `regs/standards_ready_core.rdl`.

## 26. Power Intent

N/A for this U0 FPGA example; see `power/README.md`.

## 27. DFT / Test Access

N/A for this U0 FPGA example; see `dft/README.md`.

## 28. Coding Style and Lint

The checked native lint report is stored under `reports/standards/`.

## 29. CI / Reproducibility

See `.github/workflows/ci.yml`.

## 30. Safety Hooks

See `safety/safety_manual.md`; no certification is claimed.

## 31. Security Hooks

See `security/threat_model.md`, `security/cwe_coverage.md`, and
`security/sa-edi.json`; no security certification is claimed.

## 32. HBOM / Provenance

See `hbom/standards_ready_core.spdx.json`.
