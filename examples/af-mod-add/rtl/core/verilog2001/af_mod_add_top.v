// SPDX-License-Identifier: CERN-OHL-S-2.0
`timescale 1ns/1ps
module af_mod_add_top #(
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
  output wire [FIELD_BITS-1:0] o_result
);
  af_mod_add #(
    .FIELD_BITS(FIELD_BITS),
    .MODULUS(MODULUS)
  ) u_core (
    .i_clk(i_clk),
    .i_rst_n(i_rst_n),
    .i_valid(i_valid),
    .o_ready(o_ready),
    .i_a(i_a),
    .i_b(i_b),
    .o_valid(o_valid),
    .i_ready(i_ready),
    .o_result(o_result)
  );
endmodule
