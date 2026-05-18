---
name: af-debug-portable-violation
description: Diagnose any `AF_PORTABLE_*` failure from `af core check` by reading the offending RTL, locating the exact line(s) that triggered the rule, and proposing the minimal layer-boundary refactor (move logic to `vendor/<vendor>/`, switch to ANSI Verilog-2001, wrap `initial` with sim guards, etc.). Use when `af core check` returned exit code 3 with an `AF_PORTABLE_*` code, or the user says "fix portable-Verilog violation", "why does core check reject this", "move vendor stuff out of generic core". Do NOT use for non-portable codes (those go to `af-error-explainer`).
allowed-tools: Bash, Read, Grep, Glob
---

# af-debug-portable-violation

`af`'s portable-Verilog policy is the line between "generic core that anyone can synthesise" and "vendor-locked artifact". When `af core check` fires an `AF_PORTABLE_*` code, the *cause* is structural — a marker, a keyword, a missing pragma — but the *fix* is always the same shape: move the offending thing out of the generic core into a wrapper layer, or normalise it into Verilog-2001. This skill turns a JSON failure into a one-screen patch plan.

## When to invoke

User says or pastes:

- a JSON object whose `code` matches `^AF_PORTABLE_.*$`
- "fix portable Verilog violation"
- "move vendor stuff out of generic core"
- "what's wrong with my RTL"
- "why doesn't `af core check` like this"

If `code` is `AF_*` but does NOT start with `AF_PORTABLE_`, hand off to `af-error-explainer` instead. This skill is narrow on purpose.

## Required inputs

1. **Either** the full JSON `details.issues[]` array from a failed `af core check --json`, **or** the `core_dir` path so the skill can re-run the check.
2. Optional **`vendor`** hint (e.g. `xilinx`, `intel`, `gowin`, `lattice`, `efinix`) — used when proposing the wrapper destination path (`vendor/<vendor>/`). Skill can also infer from the marker (e.g. `xpm_` → xilinx, `altsyncram` → intel, `gowin_` → gowin, `ehxpll` → lattice).

## Trigger reference

All seven `AF_PORTABLE_*` codes (source: `crates/af-rtl-inspector/src/lib.rs:197-330`).

| Code | What triggers it | Refactor target |
|---|---|---|
| `AF_PORTABLE_DEFAULT_NETTYPE_MISSING` | source text lacks `` `default_nettype none `` | Wrap the module in `` `default_nettype none `` … `` `default_nettype wire `` |
| `AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT` | identifier matches `logic`/`interface`/`modport`/`package`/`import`/`typedef`/`enum`/`struct`/`class`/`program`/`clocking`/`property`/`sequence`/`always_ff`/`always_comb`/`always_latch` | Rewrite in Verilog-2001 (`reg`/`wire`, `always @(posedge clk)`) or move module to a SystemVerilog wrapper layer outside the generic core |
| `AF_PORTABLE_VENDOR_OR_CLOCK_MARKER` | substring match for `xpm_`, `ramb`, `fifo_generator`, `fifo18`, `fifo36`, `fdre`, `oddr`, `iddr`, `altsyncram`, `scfifo`, `dcfifo`, `lpm_`, `altera_`, `intel_`, `altpll`, `mmcm`, `dcm`, `clk_wiz`, `clock_wizard`, `_pll`, `pll_`, `rpll`, `epll`, `dpll`, `clkdiv`, `bufg`, `bufio`, `gowin_`, `spx9`, `dpx9`, `sdpx9`, `ram16sdp` | Move the instantiation into `vendor/<vendor>/<wrapper>.v` and expose its signals as plain ports on the generic core |
| `AF_PORTABLE_AXI_ONLY_MARKER` | identifier or substring matches AXI signal names (`s_axi`, `m_axi`, `awvalid`, `awready`, `awaddr`, `wvalid`, `wready`, `wdata`, `wstrb`, `bvalid`, `bready`, `arvalid`, `arready`, `araddr`, `rvalid`, `rready`, `rdata`, `tvalid`, `tready`, `tdata`, `tlast`, `tkeep`, `tstrb`) | Keep the generic core on `ready`/`valid` (or AXI-Stream-neutral names); AXI mapping lives in an optional wrapper `wrapper/axi/<core>_axi.v` |
| `AF_PORTABLE_IMPLICIT_RESET` | `initial` identifier present and no recognised sim guard (`// synthesis translate_off`, `` `ifndef SYNTHESIS ``, `` `ifdef SIMULATION ``, `` `ifndef FORMAL ``) | Either drive the destination through the declared reset port, or wrap the `initial` in `// synthesis translate_off … // synthesis translate_on` (preferred) or `` `ifndef SYNTHESIS … `endif `` |
| `AF_PORTABLE_ENCRYPTED_NETLIST` | source text contains `pragma protect`/`protect begin_protected`/`protect end_protected`/`pragma protect_begin`, **or** `[sources].files` includes `.edn`/`.dcp`/`.xci`/`.qsys`/`.ipx`/`.qxp`/`.sdc` | Remove the netlist/constraint file from `[sources].files`; vendor envelopes belong in `vendor/<vendor>/` and are not portable RTL |
| `AF_PORTABLE_HARD_PHY_BLOCK` | substring match for DDR (`ddr_phy`, `ddrphy`, `lpddr`, `ddr3..5`, `mig_`, `phy_ddr`), PCIe hard IP (`pcie_phy`, `pcie3..5`, `xpcs`), MIPI (`mipi_dphy`, `mipi_csi`, `mipi_dsi`, `dphy`, `cphy`), or SerDes (`gtx_`, `gty_`, `gth_`, `gtp_`, `serdes`, `xceiver`, `lvds_serdes`) | Hard PHY blocks are out of portable-RTL scope. **Reclassify the core as `complex-vendor-aware` with `portability_level = U3` or `U4`.** Move the instantiation into `vendor/<vendor>/<core>_phy_wrapper.v` and expose only the abstract interface (lane data, status, calibration done) to the generic layer. Do not attempt to rewrite a PHY as portable RTL. |
| `AF_PORTABLE_PORT_STYLE` | top-module port declaration is missing explicit `input/output/inout` + `wire`/`reg` ANSI form | Rewrite the port list as `(input wire clk, input wire rst_n, output reg done, ...)` — one declaration per port |

The marker lists above are exact (substring matches; case-insensitive for vendor/AXI markers, token-match for SystemVerilog keywords).

## Procedure

### Step 1 — capture the failure

If JSON is provided, parse `details.issues[]`. Each item is `{code, message, hint}`. Hold on to `details.scanned_files` from the parent payload — those are the files the inspector actually read.

If only a path is provided:

```bash
cargo run --quiet -p af-cli --bin af -- core check <core_dir> --json
```

Parse the resulting `details` block.

### Step 2 — group by code

A single check can fire multiple `AF_PORTABLE_*` codes. Group issues by code. Process them in this priority order (cheap-to-fix first):

1. `AF_PORTABLE_DEFAULT_NETTYPE_MISSING` (one-line addition)
2. `AF_PORTABLE_PORT_STYLE` (header rewrite)
3. `AF_PORTABLE_IMPLICIT_RESET` (add sim guard)
4. `AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT` (keyword-by-keyword translation)
5. `AF_PORTABLE_AXI_ONLY_MARKER` (rename ports + wrapper)
6. `AF_PORTABLE_VENDOR_OR_CLOCK_MARKER` (extract vendor instantiation)
7. `AF_PORTABLE_ENCRYPTED_NETLIST` (remove source from manifest)

This ordering matters: fixing `DEFAULT_NETTYPE_MISSING` and `PORT_STYLE` first prevents the inspector from masking real issues on the second run.

### Step 3 — locate offending lines

The error itself does NOT carry line numbers. The skill must locate them. For each issue:

```bash
# Pick the marker from the message: af parser includes it as `\`<marker>\`` in `message`.
# Example message: "portable Verilog source contains forbidden marker `mmcm`"

rg --no-heading -nF '<marker>' <core_dir>/<source_file>
```

For `SYSTEMVERILOG_CONSTRUCT`, use word-boundary regex to avoid matching `wire logic_signal_name`:

```bash
rg --no-heading -n '\b<keyword>\b' <core_dir>/<source_file>
```

For `IMPLICIT_RESET`, locate every `initial` token:

```bash
rg --no-heading -nE '\binitial\b' <core_dir>/<source_file>
```

For `ENCRYPTED_NETLIST`, the offending file is the one listed in the message — no scan needed.

For `DEFAULT_NETTYPE_MISSING` and `PORT_STYLE`, the file is in `details.scanned_files` (typically the top module's primary source).

### Step 4 — propose the refactor

For each grouped code, emit the canonical patch plan (no diff syntax — describe the edit). Templates below.

#### `DEFAULT_NETTYPE_MISSING`

```
File `<path>`:
- Add `\`default_nettype none` immediately before `module <top>`.
- Add `\`default_nettype wire` immediately after the corresponding `endmodule`.
```

#### `PORT_STYLE`

Read the module header. Identify each port that lacks `input|output|inout` + `wire|reg`. Propose:

```
File `<path>:<line_of_module_decl>`:
- Rewrite the port list in ANSI Verilog-2001 form:
  `(input wire clk, input wire rst_n, output reg done, ...)`
- One direction + net type per port. No declared-then-bound form.
```

If you can read the header reliably, include the full proposed header inline (a few lines).

#### `IMPLICIT_RESET`

For each `initial` line:

```
File `<path>:<line>`:
- Option A (preferred for simulation-only init): wrap the `initial` block in
  ```
  // synthesis translate_off
  initial begin ... end
  // synthesis translate_on
  ```
- Option B (preferred for reset semantics): replace the `initial` with a
  synchronous reset assignment driven from the declared reset port.
```

#### `SYSTEMVERILOG_CONSTRUCT`

For each keyword occurrence:

| Keyword | Verilog-2001 equivalent |
|---|---|
| `logic` | `wire` (combinational) or `reg` (sequential) |
| `always_ff @(posedge clk)` | `always @(posedge clk)` |
| `always_comb` | `always @(*)` |
| `always_latch` | `always @(*)` with explicit latch comment (rarely portable) |
| `typedef enum {...} T;` | parameterised `localparam` block + plain `reg` (`[N-1:0]` wide) |
| `struct packed { ... } s_t;` | concatenation + manual bit-slicing |
| `interface`/`modport` | flatten to individual ports |
| `class`/`program` | not synthesisable; remove |
| `property`/`sequence` | move to a separate SystemVerilog-only formal file under `formal/` |

For each found instance: `File <path>:<line>: replace <kw> with <equiv>`. If the construct is `class`/`program`/`property`/`sequence`, recommend moving to `formal/<core>_props.sv` and declaring it in `[formal] files = [...]` (not in `[sources].files`).

#### `AXI_ONLY_MARKER`

```
File `<path>`:
- Rename AXI-specific ports inside the generic core to ready/valid neutrals:
  `awvalid` → `req_valid`, `awready` → `req_ready`, etc.
- Build an AXI-aware wrapper at `wrapper/axi/<core>_axi.v` that re-exposes
  the AXI signal names and routes them to the generic ports.
- Update `[sources].files` so the generic core no longer references the
  AXI signal names; the wrapper goes into a separate manifest or under
  `wrapper/` next to the core.
```

#### `VENDOR_OR_CLOCK_MARKER`

Infer vendor from marker:

| Marker prefix | Vendor |
|---|---|
| `xpm_`, `ramb`, `mmcm`, `dcm`, `clk_wiz`, `bufg`, `bufio`, `oddr`, `iddr`, `fdre` | `xilinx` |
| `altsyncram`, `altpll`, `scfifo`, `dcfifo`, `lpm_`, `altera_`, `intel_` | `intel` |
| `gowin_`, `rpll`, `epll`, `dpll`, `spx9`, `dpx9`, `sdpx9` | `gowin` |
| `ehxpll`, `ram16sdp` | `lattice` |
| `fifo_generator`, `fifo18`, `fifo36` | likely `xilinx`; user must confirm |

```
File `<path>:<line_of_marker>`:
- Move the `<marker>` instantiation into `vendor/<vendor>/<core>_<role>_wrapper.v`.
  Suggested filename: e.g. `vendor/xilinx/<core>_mmcm_wrapper.v`.
- In the generic core, replace the instantiation with a plain port:
  ```
  input wire <signal>_in,
  output wire <signal>_out,
  ```
- Update `[sources].files` to include both the generic file and the wrapper.
- Add the wrapper as a `[[backend_variants]]` entry if not present (mark `status = "planned"` if no real evidence yet).
```

If multiple vendor markers are detected from different vendors in the same generic file, that is a stronger violation — say so plainly: "this file mixes Xilinx and Intel primitives; the generic core cannot host either".

#### `ENCRYPTED_NETLIST`

The message names the offending source path. The fix is:

```
File `<core_dir>/af-core.toml`:
- Remove the line under `[sources].files` that lists the netlist/constraint
  file (e.g. `"rtl/blackbox.dcp"`).
- Move that file under `vendor/<vendor>/` (do not commit it under `rtl/`).
- If the netlist is the source of truth for some functionality, this core
  cannot be U0/U1; reclassify as U3 (single spec, vendor backend) and
  re-architect the manifest accordingly.

If the trigger was `pragma protect` inside an actual .v/.sv file:
- The file is itself encrypted-IP. It cannot live in `rtl/`. Remove from
  `[sources].files`; if the original-source-of-truth is unavailable, the
  core cannot be portable.
```

### Step 5 — verify (suggest, do not run)

After the user applies the proposed fixes, suggest exactly:

```bash
cargo run --quiet -p af-cli --bin af -- core check <core_dir> --json
```

Stop. Do not re-run the check yourself in a loop — the user iterates.

## Required output

```
## Portable-Verilog violations — <core_dir>

<N> issue(s) across <M> file(s).

## Fix plan (cheap → expensive)

### 1. `<CODE>` × <count>

File `<path>:<line>` — marker `<marker>`:

<canonical refactor template from above, with paths substituted>

(repeat for each grouped code in priority order)

## Vendor inferred

| File | Marker | Vendor | Suggested wrapper path |
|---|---|---|---|
| `<path>` | `<marker>` | `<vendor>` | `vendor/<vendor>/<core>_<role>_wrapper.v` |

## Verify

```bash
cargo run --quiet -p af-cli --bin af -- core check <core_dir> --json
```
```

If only one code fires and it is trivial (`DEFAULT_NETTYPE_MISSING` alone), the output is just one short section — no need for the "× count" header.

## Hard rules

- **Never edit RTL automatically.** This skill describes the patch; the user applies it. Avoiding auto-edits keeps the human in the loop on real RTL changes.
- **Never propose raising the manifest's language to `systemverilog`** as a fix for `AF_PORTABLE_SYSTEMVERILOG_CONSTRUCT`. The manifesto rejects SystemVerilog in generic cores. The legitimate options are: rewrite in Verilog-2001, or relocate to a SystemVerilog wrapper outside the generic-core layer.
- **Never suggest disabling the inspector** or any `// af-disable` pragma. The check is policy.
- **Never bring vendor netlists into `rtl/`.** Always direct them to `vendor/<vendor>/`.
- **Never speculate about line numbers.** If `rg` returns zero matches for a marker the parser caught, say so plainly: "the inspector flagged `<marker>` but it is not literally present in any source file — investigate generated/include paths and `[sources].include_dirs`".
- **Stay under one screen per violation.** A 7-line patch plan beats a 70-line essay every time.
- **Hand off non-portable codes.** If the user pastes a JSON with codes outside `AF_PORTABLE_*`, immediately delegate to `af-error-explainer` and stop.

## Worked example

User pastes:

```json
{
  "code": "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER",
  "message": "portable Verilog source contains forbidden marker `mmcm`",
  "hint": "Keep vendor primitives, hard macros, PLLs, clock dividers, and board-specific adaptation outside the generic core.",
  "exit_code": 3,
  "details": {
    "scanned_files": ["rtl/af_clk_top.v", "rtl/af_clk_bridge.v"],
    "issues": [
      {"code": "AF_PORTABLE_VENDOR_OR_CLOCK_MARKER", "message": "...`mmcm`...", "hint": "..."},
      {"code": "AF_PORTABLE_DEFAULT_NETTYPE_MISSING", "message": "...", "hint": "..."}
    ]
  }
}
```

Skill runs:

```bash
rg --no-heading -nF 'mmcm' rtl/af_clk_top.v rtl/af_clk_bridge.v
```

Output (paraphrased):

```
## Portable-Verilog violations — .

2 issue(s) across 2 file(s).

## Fix plan (cheap → expensive)

### 1. `AF_PORTABLE_DEFAULT_NETTYPE_MISSING`

File `rtl/af_clk_top.v`:
- Add `\`default_nettype none` before `module af_clk_top`.
- Add `\`default_nettype wire` after `endmodule`.
- Same for `rtl/af_clk_bridge.v`.

### 2. `AF_PORTABLE_VENDOR_OR_CLOCK_MARKER`

File `rtl/af_clk_top.v:42` — marker `mmcm`:
- Move the MMCME2_BASE instantiation into `vendor/xilinx/af_clk_top_mmcm_wrapper.v`.
- Replace the in-module instantiation with three plain ports:
  `input wire clk_out_p, input wire mmcm_locked, output wire ref_clk`.
- Update `[sources].files`:
  - generic: `rtl/af_clk_top.v`, `rtl/af_clk_bridge.v`
  - vendor: add `vendor/xilinx/af_clk_top_mmcm_wrapper.v`
- Declare a `[[backend_variants]]` with `name = "xilinx_7series", vendor = "xilinx", status = "planned"`.

## Vendor inferred

| File | Marker | Vendor | Suggested wrapper path |
|---|---|---|---|
| `rtl/af_clk_top.v` | `mmcm` | `xilinx` | `vendor/xilinx/af_clk_top_mmcm_wrapper.v` |

## Verify

```bash
cargo run --quiet -p af-cli --bin af -- core check . --json
```
```

Match this shape.
