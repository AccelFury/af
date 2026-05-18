// SPDX-License-Identifier: CERN-OHL-S-2.0
`timescale 1ns/1ps
module tb_af_mod_add;
  import tb_pkg::*;

  logic                  clk;
  logic                  rst_n;
  logic                  i_valid;
  logic                  o_ready;
  logic [tb_pkg::FIELD_BITS-1:0] i_a;
  logic [tb_pkg::FIELD_BITS-1:0] i_b;
  logic                  o_valid;
  logic                  i_ready;
  logic [tb_pkg::FIELD_BITS-1:0] o_result;

  typedef struct packed {
    logic [tb_pkg::FIELD_BITS-1:0] a;
    logic [tb_pkg::FIELD_BITS-1:0] b;
    logic [tb_pkg::FIELD_BITS-1:0] exp;
  } vector_t;

  localparam int BASIC_VECTOR_COUNT = 4;
  const vector_t BASIC_VECTORS [BASIC_VECTOR_COUNT] = '{
    '{64'h0, 64'h0, 64'h0},
    '{64'h1, 64'h1, 64'h2},
    '{64'hFFFF_FFFF_0000_0000, 64'h1, 64'h0},
    '{64'hFFFF_FFFF_0000_0000, 64'hFFFF_FFFF_0000_0000, 64'hFFFF_FFFF_0000_0000 - 1}
  };

  `include "vectors/af_mod_add_random.svh"

  af_mod_add_top #(
    .FIELD_BITS(tb_pkg::FIELD_BITS),
    .MODULUS(tb_pkg::MODULUS)
  ) dut (
    .i_clk(clk),
    .i_rst_n(rst_n),
    .i_valid(i_valid),
    .o_ready(o_ready),
    .i_a(i_a),
    .i_b(i_b),
    .o_valid(o_valid),
    .i_ready(i_ready),
    .o_result(o_result)
  );

  always #5 clk = ~clk;

  initial begin
    clk = 1'b0;
    rst_n = 1'b0;
    i_valid = 1'b0;
    i_a = '0;
    i_b = '0;
    i_ready = 1'b0;
    repeat (3) @(posedge clk);
    rst_n = 1'b1;
    @(posedge clk);

    i_ready = 1'b1;
    drive_vectors();
    drive_random_vectors();
    @(posedge clk);
    if (!all_ok) begin
      $fatal("tb_af_mod_add failed");
    end
    $display("tb_af_mod_add passed");
    $finish;
  end

  bit all_ok = 1'b1;

  task drive_one(input vector_t v);
    begin
      @(posedge clk);
      while (!o_ready) @(posedge clk);
      i_a <= v.a;
      i_b <= v.b;
      i_valid <= 1'b1;
      @(posedge clk);
      i_valid <= 1'b0;
      if (!o_valid) begin
        wait (o_valid);
      end
      if (o_result !== v.exp) begin
        all_ok = 1'b0;
        $display("Mismatch a=%h b=%h exp=%h got=%h", v.a, v.b, v.exp, o_result);
        $fatal(1, "Mismatch in DUT output");
      end
      if (o_ready !== 1'b1) begin
        if (o_result !== v.exp) begin
          all_ok = 1'b0;
          $fatal("Output changed while stalled");
        end
      end
    end
  endtask

  task drive_vectors();
    int i;
    vector_t v;
    begin
      for (i = 0; i < BASIC_VECTOR_COUNT; i++) begin
        v = BASIC_VECTORS[i];
        drive_one(v);
      end
    end
  endtask

  task drive_random_vectors();
    int i;
    vector_t v;
    begin
      for (i = 0; i < AF_MOD_ADD_RANDOM_COUNT; i++) begin
        v = '{AF_MOD_ADD_RANDOM_A[i], AF_MOD_ADD_RANDOM_B[i], AF_MOD_ADD_RANDOM_EXPECTED[i]};
        drive_one(v);
      end
    end
  endtask
endmodule
