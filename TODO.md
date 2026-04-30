# TODO - AccelFury `af`

This is the active backlog for making `af` the primary FPGA core development,
check, debug, packaging, and CI/CD tool.

- [ ] **Icarus/Yosys/nextpnr/SymbiYosys backends.** Add first-class backends for
      Verilog simulation, synthesis, place-and-route, timing/resource report
      capture, and formal checks. Critical because buyer-grade FPGA IP cannot
      rely only on manifest checks and optional Verilator smoke.
- [ ] **Report ingestion and release evidence gates.** Add commands that import
      simulator logs, lint transcripts, formal verdicts, synthesis reports,
      PnR timing/resource JSON, programming logs, and hardware measurement
      evidence into normalized reports. Critical because external integrators
      need reproducible evidence rather than prose claims.
- [ ] **Manifest migration and compatibility diagnostics.** Provide a migration
      command for older project-local manifests and keep parse errors
      actionable when required v0.2 fields are missing. Critical because mature
      cores such as `af-pdm-rx` may already have project metadata that predates
      the current `af-core.toml` schema.
- [ ] **Wrapper targets beyond FuseSoC.** Add IP-XACT, LiteX, AXI4-Stream,
      AXI-Lite, Wishbone, and vendor-project wrapper/export targets. Critical
      because buyer-grade cores must integrate cleanly into diverse FPGA
      projects and toolchains.
- [ ] **CI matrix generation.** Extend `af ci generate` beyond MVP GitHub
      Actions smoke so it can emit backend-aware job matrices, artifact upload,
      report publishing, and fail-closed optional-tool handling. Critical
      because `af` should be usable as a CI/CD tool, not only a local checker.
