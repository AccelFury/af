// SPDX-License-Identifier: CERN-OHL-S-2.0
`timescale 1ns/1ps
module af_mod_add #(
  parameter integer FIELD_BITS = 64,
  parameter [FIELD_BITS-1:0] MODULUS = 64'hFFFF_FFFF_0000_0001
) (
  input  wire                  i_clk,
  input  wire                  i_rst_n,
  input  wire                  i_valid,
  output wire                  o_ready,
  input  wire [FIELD_BITS-1:0] i_a,
  input  wire [FIELD_BITS-1:0] i_b,
  output wire                  o_valid,
  input  wire                  i_ready,
  output reg  [FIELD_BITS-1:0] o_result
);
  wire [FIELD_BITS:0] sum_full;
  reg  [FIELD_BITS-1:0] sum_mod;
  wire accept;
  reg  full;

  assign o_ready = i_ready || !full;
  assign accept = i_valid && o_ready;
  assign o_valid = full;
  assign sum_full = {1'b0, i_a} + {1'b0, i_b};

  always @* begin
    if (sum_full >= {1'b0, MODULUS}) begin
      sum_mod = sum_full[FIELD_BITS-1:0] - MODULUS;
    end else begin
      sum_mod = sum_full[FIELD_BITS-1:0];
    end
  end

  always @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      full <= 1'b0;
      o_result <= {FIELD_BITS{1'b0}};
    end else begin
      if (accept) begin
        full <= 1'b1;
        o_result <= sum_mod;
      end else if (i_ready) begin
        full <= 1'b0;
      end
    end
  end
endmodule
