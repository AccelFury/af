// SPDX-License-Identifier: LicenseRef-AccelFury-Source-Available-1.0
//
// N-stage reset synchronizer with configurable polarity.
//
// Asynchronous assertion, synchronous deassertion. The destination reset
// follows the source reset polarity. The synchronizer is intentionally
// portable Verilog-2001: no vendor primitives, no PLL, no clock managers.
//
// Verification gates declared in af-core.toml:
//   - formal-cdc-assumption (formal-style equivalence of N-stage shift)
//   - simulation (smoke testbench under examples/af-reset-sync/tb)
//
// Parameters:
//   STAGES          Number of synchronizer flops in `clk` domain (>=2).
//   RESET_POLARITY  0 = active-low (src_rst_n / dst_rst_n);
//                   1 = active-high (src_rst / dst_rst).

`default_nettype none

module af_reset_sync #(
    parameter STAGES          = 2,
    parameter RESET_POLARITY  = 0
) (
    input  wire clk,
    input  wire src_rst,
    output wire dst_rst
);

    // synthesis translate_off
    initial begin
        if (STAGES < 2) begin
            $display("af_reset_sync: STAGES must be >= 2 (got %0d)", STAGES);
            $finish;
        end
    end
    // synthesis translate_on

    reg [STAGES-1:0] sync_chain;

    // Asynchronous assert / synchronous deassert. The clause structure differs
    // by polarity so we keep the two flavors explicit instead of XORing.
    generate
        if (RESET_POLARITY == 0) begin : g_active_low
            always @(posedge clk or negedge src_rst) begin
                if (!src_rst) begin
                    sync_chain <= {STAGES{1'b0}};
                end else begin
                    sync_chain <= {sync_chain[STAGES-2:0], 1'b1};
                end
            end
            assign dst_rst = sync_chain[STAGES-1];
        end else begin : g_active_high
            always @(posedge clk or posedge src_rst) begin
                if (src_rst) begin
                    sync_chain <= {STAGES{1'b1}};
                end else begin
                    sync_chain <= {sync_chain[STAGES-2:0], 1'b0};
                end
            end
            assign dst_rst = sync_chain[STAGES-1];
        end
    endgenerate

endmodule

`default_nettype wire
