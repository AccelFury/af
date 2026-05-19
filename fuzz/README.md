# Fuzzing

This directory contains manual/nightly cargo-fuzz targets for parser and
contract surfaces that are too broad for example-based tests.

Run from the repository root:

```bash
cargo +nightly fuzz run manifest_toml -- -runs=1024
cargo +nightly fuzz run security_paths -- -runs=1024
cargo +nightly fuzz run board_registry_json -- -runs=1024
cargo +nightly fuzz run rtl_source -- -runs=1024
cargo +nightly fuzz run ci_generate_options -- -runs=1024
```

The targets must stay offline and must not require vendor tools, hardware, or
network access. Generated corpora, artifacts, and coverage output are ignored.
