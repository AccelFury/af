// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_board_top (
  input  wire        i_clk,
  input  wire        i_rst_n,
  output reg [1:0]   led
);
  localparam int HEARTBEAT_DIV = 22;
  reg  [HEARTBEAT_DIV:0] heartbeat_cnt;
  reg  [63:0] i_a;
  reg  [63:0] i_b;
  reg         i_valid;
  reg         self_test_done;
  wire        o_ready;
  wire        o_valid;
  wire [63:0] o_result;

  af_mod_add_top #(
    .FIELD_BITS(64),
    .MODULUS(64'hFFFF_FFFF_0000_0001)
  ) u_core (
    .i_clk(i_clk),
    .i_rst_n(i_rst_n),
    .i_valid(i_valid),
    .o_ready(),
    .i_a(i_a),
    .i_b(i_b),
    .o_valid(o_valid),
    .i_ready(1'b1),
    .o_result(o_result)
  );

  always @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      heartbeat_cnt <= '0;
      i_a <= 64'h0;
      i_b <= 64'h0;
      i_valid <= 1'b0;
      self_test_done <= 1'b0;
      led <= 2'b00;
    end else begin
      heartbeat_cnt <= heartbeat_cnt + 1'b1;
      if (!self_test_done) begin
        i_a <= 64'h1;
        i_b <= 64'h1;
        i_valid <= 1'b1;
        if (o_valid) begin
          self_test_done <= 1'b1;
          i_valid <= 1'b0;
          led <= (o_result == 64'h2) ? 2'b11 : 2'b01;
        end
      end else begin
        i_valid <= 1'b0;
        led <= {heartbeat_cnt[HEARTBEAT_DIV-1], heartbeat_cnt[HEARTBEAT_DIV-2]};
      end
    end
  end
endmodule
