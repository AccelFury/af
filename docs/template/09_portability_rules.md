# Portability Rules

1. Core files cannot include vendor primitives.
2. Board-specific I/O mapping and reset assumptions only in `boards/*`.
3. Vendor primitive wrappers only in `rtl/vendor/*`.
4. Top-level ready/valid protocol names are stable across IP derivatives.
5. Do not reference unsupported/non-portable clock generators in core.
6. Portability baseline languages are Verilog-2001, Verilog-2005, and
   SystemVerilog.
7. Keep `.md`, `.sv`, `.v`, `.rs`, `.ts` files with SPDX headers.
8. A build must select one implementation language lane for each module name: do
   not compile `rtl/core/af_mod_add.sv` and `rtl/core/verilog2001/af_mod_add.v`
   in the same filelist.
9. Verilog-2001/2005 sources avoid `logic`, `always_ff`, `always_comb`,
   packages, interfaces, typedefs, packed structs, assertions, and parameter
   types such as `parameter int`.
10. SystemVerilog sources may use a conservative synthesizable subset, but
    public protocol, reset, latency and backpressure semantics must match the
    Verilog implementation.
