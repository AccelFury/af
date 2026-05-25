# Reproducible Builds

Release artifacts are built from an exact commit and verified by
`af release check --json`.

## Local binary

```bash
git checkout <commit-sha>
rustup toolchain install 1.95.0-x86_64-unknown-linux-gnu
export RUSTUP_TOOLCHAIN=1.95.0-x86_64-unknown-linux-gnu
cargo build --locked --release -p af-cli --bin af
mkdir -p .af-build/release/artifacts
tar -czf .af-build/release/artifacts/af-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz \
  -C target/release af
(cd .af-build/release/artifacts && sha256sum af-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz > SHA256SUMS)
```

Verify:

```bash
(cd .af-build/release/artifacts && sha256sum -c SHA256SUMS)
```

## Docker image

The release workflow publishes:

```text
ghcr.io/accelfury/af:<tag>
ghcr.io/accelfury/af@sha256:<digest>
```

The immutable digest and smoke report path are recorded in
`.af-build/release/docker-image.json`.

## Release gate

```bash
af release check \
  --tag v0.2.0-rc.1 \
  --ci-evidence .af-build/release/ci-evidence.json \
  --artifact-dir .af-build/release/artifacts \
  --docker-evidence .af-build/release/docker-image.json \
  --output .af-build/release/release-readiness.json \
  --json
```

The gate fails closed if the source tree is dirty or if external CI evidence,
checksums, Docker digest, smoke evidence, or claim discipline is missing.
