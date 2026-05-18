// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_mod_add_top #(
  parameter int FIELD_BITS = 64,
  parameter logic [FIELD_BITS-1:0] MODULUS = 64'hFFFF_FFFF_0000_0001
) (
  input  logic                  i_clk,
  input  logic                  i_rst_n,
  input  logic                  i_valid,
  output logic                  o_ready,
  input  logic [FIELD_BITS-1:0] i_a,
  input  logic [FIELD_BITS-1:0] i_b,
  output logic                  o_valid,
  input  logic                  i_ready,
  output logic [FIELD_BITS-1:0] o_result
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
