// SPDX-License-Identifier: LicenseRef-AccelFury-Source-Available-1.0
//
// Smoke testbench for af_reset_sync.
//
// Verifies:
//   1. After src_rst is asserted, dst_rst follows the source polarity within
//      one clock edge (async assert).
//   2. After src_rst is released, dst_rst releases only after STAGES rising
//      edges (sync deassert).
//
// This is a portable Icarus/Verilator-compatible testbench. It does not claim
// formal CDC sign-off; that gate is declared in af-core.toml under
// [[verification_required]] kind = "formal-cdc-assumption".

`timescale 1ns/1ps

module tb_af_reset_sync;
    localparam integer STAGES = 3;

    reg  clk;
    reg  src_rst_n;
    wire dst_rst_n;

    integer cycles_after_release;
    integer errors;

    af_reset_sync #(
        .STAGES         (STAGES),
        .RESET_POLARITY (0)
    ) u_dut (
        .clk     (clk),
        .src_rst (src_rst_n),
        .dst_rst (dst_rst_n)
    );

    initial clk = 1'b0;
    always #5 clk = ~clk;

    initial begin
        errors                = 0;
        cycles_after_release  = 0;
        src_rst_n             = 1'b0;

        // 1) Asynchronous assert: dst_rst_n must be low while src_rst_n is low.
        #2;
        if (dst_rst_n !== 1'b0) begin
            $display("FAIL: async assert did not propagate (dst_rst_n=%0b)", dst_rst_n);
            errors = errors + 1;
        end

        // Hold reset over a few clocks.
        @(posedge clk); @(posedge clk);

        // 2) Synchronous deassert: release at a known edge and count cycles
        //    until dst_rst_n rises.
        @(negedge clk);
        src_rst_n = 1'b1;

        begin : observation
            forever begin
                @(posedge clk);
                cycles_after_release = cycles_after_release + 1;
                if (dst_rst_n === 1'b1) disable observation;
                if (cycles_after_release > STAGES + 4) begin
                    $display("FAIL: dst_rst_n did not release within STAGES+4 cycles");
                    errors = errors + 1;
                    disable observation;
                end
            end
        end
        if (cycles_after_release < STAGES) begin
            $display("FAIL: dst_rst_n released too early after %0d cycles (STAGES=%0d)",
                     cycles_after_release, STAGES);
            errors = errors + 1;
        end

        if (errors == 0)
            $display("PASS: af_reset_sync smoke (STAGES=%0d, RESET_POLARITY=0)", STAGES);
        else
            $display("SMOKE FAILED with %0d error(s)", errors);
        $finish;
    end
endmodule
