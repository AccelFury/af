<!-- SPDX-License-Identifier: Apache-2.0 -->

# Simulation Plan

Planned scenarios:

- reset clears `m_valid`, `m_data`, and `counter_words`;
- one accepted word appears at output after one cycle;
- backpressure holds `m_valid` and `m_data`;
- `cfg_enable = 0` deasserts `s_ready`.

This reference example records the plan; it does not claim completed simulation
coverage.
