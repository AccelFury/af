// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_skid_buffer #(
  parameter int DATA_WIDTH = 64
) (
  input  logic                  i_clk,
  input  logic                  i_rst_n,
  input  logic                  i_valid,
  output logic                  o_ready,
  input  logic [DATA_WIDTH-1:0] i_data,
  output logic                  o_valid,
  input  logic                  i_ready,
  output logic [DATA_WIDTH-1:0] o_data
);
  logic full;
  logic [DATA_WIDTH-1:0] buffer;

  assign o_ready = !full || i_ready;
  assign o_valid = full;

  always_ff @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      full <= 1'b0;
      buffer <= '0;
    end else begin
      if (o_ready && i_valid) begin
        buffer <= i_data;
        full <= 1'b1;
      end else if (i_ready) begin
        full <= 1'b0;
      end
    end
  end

  assign o_data = buffer;
endmodule
