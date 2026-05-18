// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_mod_add #(
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
  logic [FIELD_BITS:0] sum_full;
  logic [FIELD_BITS-1:0] sum_minus_mod;
  logic               accept;
  logic               full;

  assign o_ready = i_ready || !full;
  assign accept = i_valid && o_ready;
  assign o_valid = full;

  always_comb begin
    sum_full = '0;
    sum_minus_mod = '0;
    sum_full = {1'b0, i_a} + {1'b0, i_b};
    if (sum_full >= {1'b0, MODULUS}) begin
      sum_minus_mod = sum_full[FIELD_BITS-1:0] - MODULUS;
    end else begin
      sum_minus_mod = sum_full[FIELD_BITS-1:0];
    end
  end

  always_ff @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      full      <= 1'b0;
      o_result  <= '0;
    end else begin
      if (accept) begin
        full     <= 1'b1;
        o_result <= sum_minus_mod[FIELD_BITS-1:0];
      end else if (i_ready) begin
        full <= 1'b0;
      end
    end
  end

  // synthesis translate_off
  /* verilator lint_off SYNCASYNCNET */
  property p_no_x_inputs;
    @(posedge i_clk) disable iff (!i_rst_n)
      $isunknown(i_a) === 0 && $isunknown(i_b) === 0;
  endproperty
  assert property (p_no_x_inputs) else $error("af_mod_add: X inputs");

  property p_output_stable_when_blocked;
    @(posedge i_clk) disable iff (!i_rst_n)
      (full && !i_ready) |=> (o_result === $past(o_result));
  endproperty
  assert property (p_output_stable_when_blocked)
    else $error("af_mod_add: output must be stable while not ready");
  /* verilator lint_on SYNCASYNCNET */
  // synthesis translate_on
endmodule
