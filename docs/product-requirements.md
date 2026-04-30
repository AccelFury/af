# Product Requirements

AccelFury `af` turns handwritten FPGA/IP cores into repeatable engineering
packages with manifests, checks, wrappers, reports, and CI.

Target users:

- HDL engineers packaging reusable cores;
- board maintainers adding constrained target profiles;
- tooling engineers adding backend adapters;
- CI maintainers who need machine-readable reports.

Success metrics:

- `examples/af-pdm-rx` passes manifest/core checks on a clean checkout;
- optional external tools report structured unavailable status;
- generated wrappers/reports are deterministic and marked generated;
- no generated file silently creates CDC, FIFO, filter, or bus bridge logic.

Non-goals:

- timing signoff;
- full SystemVerilog parsing;
- vendor bitstream production by default;
- PDM-to-PCM audio conversion in `af-pdm-rx`.
