// SPDX-License-Identifier: LicenseRef-AccelFury-Source-Available-1.0
//
// Formal property stub for af_reset_sync. The stub is intentionally minimal
// and marked enabled=false in af-core.toml so the property file is tracked
// as a verification gate, not as a claim. Engage SymbiYosys with vendor
// reset assumptions before promoting the gate from "preview" to "stable".

`default_nettype none

module af_reset_sync_props #(
    parameter STAGES         = 2,
    parameter RESET_POLARITY = 0
) (
    input wire clk,
    input wire src_rst,
    input wire dst_rst
);

`ifdef FORMAL
    // Async assertion: while src_rst is in its asserted level, dst_rst must
    // be in the same asserted level after at most one clock edge.
    generate
        if (RESET_POLARITY == 0) begin : g_active_low
            always @(posedge clk) begin
                if (!src_rst) assert (!dst_rst);
            end
        end else begin : g_active_high
            always @(posedge clk) begin
                if (src_rst) assert (dst_rst);
            end
        end
    endgenerate
`endif

endmodule

`default_nettype wire
