# Docker Runtime

The project uses a Docker open-source runtime to make `af` validation repeatable
when the host does not have FPGA tools in `PATH`.

The runtime installs:

- Rust stable for building the CLI inside the container;
- Verilator for `af core lint` and `af core sim`;
- xmllint for XML/IP-XACT/package metadata checks;
- FuseSoC and Edalize for package visibility checks and `.core` integration
  workflows;
- optional LiteX Python package when built with
  `--build-arg AF_INSTALL_LITEX=true`;
- Yosys for syntax/synthesis smoke checks;
- SMT solvers for formal/SymbiYosys flows: Boolector, Z3, Yices (`yices-smt2`),
  Bitwuzla and cvc5;
- optional distro packages for SymbiYosys and openFPGALoader when available.

Generated files remain limited to wrappers, manifest/package exports, build
scripts, CI files and reports. Handwritten RTL stays the source of hardware
logic.

## Commands

Use the published release image when validating a released tag:

```bash
docker run --rm -v "$PWD:/work" -w /work ghcr.io/accelfury/af@sha256:<digest> scripts/docker-smoke.sh
```

The digest is recorded in `.af-build/release/docker-image.json` by the release
workflow and is consumed by `af release check --json`.

Build the runtime:

```bash
make docker-build
```

Inspect or build the runtime through the CLI tooling planner:

```bash
af tooling plan --profile oss --install-mode docker --allow-network --json
af tooling ensure --profile oss --install-mode docker --allow-network --yes
```

Run the full Docker smoke:

```bash
make docker-smoke
```

The Makefile mounts the host Cargo registry and a `/tmp/af-docker-target` target
cache. This keeps the in-container Rust build deterministic without runtime
`rustup` channel sync and without repeated crates.io downloads after the first
cached build.

The smoke covers:

- `af doctor --json`;
- legacy manifest migration dry-run;
- `af core check`;
- Verilator lint and smoke simulation;
- xmllint, FuseSoC, and Edalize tool visibility;
- FuseSoC `.core` generation;
- LiteX wrapper/reference dry-run through the Rust skeleton generator;
- Yosys syntax/synthesis smoke;
- SMT solver binary visibility for Boolector, Z3, Yices, Bitwuzla and cvc5;
- package/report generation.

Artifacts are written under `.af-build/docker-smoke/`.

## Boundaries

The Docker runtime is the primary debug surface for open-source tools. Vendor
EDA tools remain host/local-runner integrations because licenses, installers and
EULAs are outside the distributable container boundary. Use
`docs/vendor-tooling.md` to bind-mount an already installed private vendor tree
into a local hardware-runner container.

LiteX Python installation is optional in the default image because the MVP
currently generates a skeleton/reference wrapper and does not execute a LiteX
SoC build. To force-install LiteX for local experiments:

```bash
docker build --build-arg AF_INSTALL_LITEX=true -t accelfury-af:oss-litex .
```

The default Docker build installs the full SMT solver set. To build a smaller
image that installs only distro-packaged solvers and allows missing
`yices-smt2`/`bitwuzla` on distributions that do not package them:

```bash
docker build --build-arg AF_INSTALL_FULL_SMT_SOLVERS=false -t accelfury-af:oss-min .
```
