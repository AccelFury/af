# Board Author Guide

Board profiles describe available resources without implying hardware
validation. Any pin/resource claim that is not bench-validated must set
`verified = false`.

Minimum `af-board.toml` content:

- `schema_version` and `kind`;
- `[name]` id and display name;
- `[fpga]` vendor, family, part/package when known;
- default clock with `verified`;
- programming backend hints;
- constraints format and default file;
- resources and caveats.

Tang Nano 20K is the primary MVP board profile. Tang Primer 20K is experimental.
