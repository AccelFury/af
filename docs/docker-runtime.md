# Docker Runtime

The project uses a Docker open-source runtime to make `af` validation
repeatable when the host does not have FPGA tools in `PATH`.

The runtime installs:

- Rust stable with `rustfmt` and `clippy`;
- Verilator for `af core lint` and `af core sim`;
- FuseSoC for package visibility checks and `.core` validation workflows;
- LiteX as a Python module for reference wrapper integration work;
- Yosys for syntax/synthesis smoke checks;
- optional distro packages for SymbiYosys and openFPGALoader when available.

Generated files remain limited to wrappers, manifest/package exports, build
scripts, CI files and reports. Handwritten RTL stays the source of hardware
logic.

## Commands

Build the runtime:

```bash
docker build -t accelfury-af:oss .
```

Run the full Docker smoke:

```bash
docker run --rm -v "$PWD:/work" -w /work \
  -e AF_BUILD_ROOT=/work/.af-build/docker-smoke \
  accelfury-af:oss scripts/docker-smoke.sh
```

The smoke covers:

- `af doctor --json`;
- legacy manifest migration dry-run;
- `af core check`;
- Verilator lint and smoke simulation;
- FuseSoC `.core` generation;
- LiteX wrapper/reference dry-run;
- Yosys syntax/synthesis smoke;
- package/report generation.

Artifacts are written under `.af-build/docker-smoke/`.

## Boundaries

The Docker runtime is the primary debug surface for open-source tools. Vendor
EDA tools remain host/local-runner integrations because licenses, installers and
EULAs are outside the distributable container boundary.
