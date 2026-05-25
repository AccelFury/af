<!-- SPDX-License-Identifier: Apache-2.0 -->

# Formal Verification Plan

Planned properties:

- no output data changes while `m_valid && !m_ready`;
- `counter_words` changes only on accepted input;
- reset returns the core to the empty state.

No formal proof is claimed for this reference example.
