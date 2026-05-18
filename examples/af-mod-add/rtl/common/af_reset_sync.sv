// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_reset_sync (
  input  logic clk,
  input  logic i_async_rst_n,
  output logic o_sync_rst_n
);
  logic ff1;
  logic ff2;

  always_ff @(posedge clk or negedge i_async_rst_n) begin
    if (!i_async_rst_n) begin
      ff1 <= 1'b0;
      ff2 <= 1'b0;
    end else begin
      ff1 <= 1'b1;
      ff2 <= ff1;
    end
  end

  assign o_sync_rst_n = ff2;
endmodule
