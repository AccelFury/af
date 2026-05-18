// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_cdc_pulse #(
  parameter int SYNC_STAGES = 2
) (
  input  logic i_src_clk,
  input  logic i_src_rst_n,
  input  logic i_pulse_src,
  input  logic i_dst_clk,
  input  logic i_dst_rst_n,
  output logic o_pulse_dst
);
  logic [SYNC_STAGES-1:0] token;
  logic                  src_level;

  always_ff @(posedge i_src_clk or negedge i_src_rst_n) begin
    if (!i_src_rst_n) begin
      src_level <= 1'b0;
    end else if (i_pulse_src) begin
      src_level <= ~src_level;
    end
  end

  always_ff @(posedge i_dst_clk or negedge i_dst_rst_n) begin
    if (!i_dst_rst_n) begin
      token <= '0;
    end else begin
      token <= {token[SYNC_STAGES-2:0], src_level};
    end
  end

  assign o_pulse_dst = token[SYNC_STAGES-1] ^ token[SYNC_STAGES-2];
endmodule
