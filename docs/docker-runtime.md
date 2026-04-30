# Docker Runtime

The project uses a Docker open-source runtime to make `af` validation
repeatable when the host does not have FPGA tools in `PATH`.

The runtime installs:

- Rust stable with `rustfmt` and `clippy`;
- Verilator for `af core lint` and `af core sim`;
- FuseSoC for package visibility checks and `.core` validation workflows;
- optional LiteX Python package when built with `--build-arg AF_INSTALL_LITEX=true`;
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
- LiteX wrapper/reference dry-run through the Rust skeleton generator;
- Yosys syntax/synthesis smoke;
- package/report generation.

Artifacts are written under `.af-build/docker-smoke/`.

## Boundaries

The Docker runtime is the primary debug surface for open-source tools. Vendor
EDA tools remain host/local-runner integrations because licenses, installers and
EULAs are outside the distributable container boundary.

LiteX Python installation is optional in the default image because the MVP
currently generates a skeleton/reference wrapper and does not execute a LiteX
SoC build. To force-install LiteX for local experiments:

```bash
docker build --build-arg AF_INSTALL_LITEX=true -t accelfury-af:oss-litex .
```
