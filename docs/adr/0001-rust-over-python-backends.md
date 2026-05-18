# ADR 0001: Rust Orchestrator Over Python Backends

Rust owns the CLI, manifest validation, security checks, command planning, and
reporting. Python-based tools such as LiteX remain external optional tools.

This keeps command policy and report contracts stable while avoiding rewrites of
existing FPGA ecosystems.
