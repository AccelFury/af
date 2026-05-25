<!-- SPDX-License-Identifier: Apache-2.0 -->

# standards-ready-core

Minimal portable core used to demonstrate the `fpga-ip-core-v1` standards
evidence layout.

Useful checks:

```sh
af manifest validate examples/standards-ready-core/af-core.toml --json
af core check examples/standards-ready-core --json
af core regs check examples/standards-ready-core --json
af core standards spdx-audit examples/standards-ready-core --json
af core standards check examples/standards-ready-core --profile fpga-ip-core-v1 --strict --json
```

This example is not safety-certified, security-certified, timing-signed-off, or
validated on hardware.
