// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_dsp_mul #(
  parameter int WIDTH = 32
) (
  input  logic                  i_clk,
  input  logic                  i_rst_n,
  input  logic [WIDTH-1:0]      i_a,
  input  logic [WIDTH-1:0]      i_b,
  output logic [2*WIDTH-1:0]    o_result
);
  always_ff @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      o_result <= '0;
    end else begin
      o_result <= i_a * i_b;
    end
  end
endmodule
