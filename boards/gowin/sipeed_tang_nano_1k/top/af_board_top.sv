// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_board_top (
  input  wire       i_clk,
  input  wire       i_rst_n,
  output reg [1:0]  led
);
  localparam [1:0] LED_OK = 2'b11;
  localparam [1:0] LED_FAIL = 2'b01;
  localparam [1:0] LED_IDLE = 2'b00;

  logic        self_test_done;
  logic        self_test_ok;
  logic [31:0] heartbeat;

  logic        i_valid;
  logic [63:0] i_a;
  logic [63:0] i_b;
  logic        o_valid;
  logic        i_ready;
  logic [63:0] o_result;

  // UART-free smoke test runs during power-up.
  assign i_ready = 1'b1;
  assign i_a = self_test_done ? 64'h0 : 64'h0000_0000_0000_0001;
  assign i_b = self_test_done ? 64'h0 : 64'h0000_0000_0000_0001;
  assign i_valid = self_test_done ? 1'b0 : 1'b1;

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
    .i_ready(i_ready),
    .o_result(o_result)
  );

  always_ff @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      self_test_done <= 1'b0;
      self_test_ok   <= 1'b0;
      heartbeat      <= 32'h0;
      led            <= LED_IDLE;
    end else begin
      heartbeat <= heartbeat + 1'b1;
      if (!self_test_done && o_valid) begin
        self_test_done <= 1'b1;
        self_test_ok   <= (o_result == 64'h2);
        led <= (o_result == 64'h2) ? LED_OK : LED_FAIL;
      end else if (self_test_done) begin
        led <= self_test_ok ? {heartbeat[23], ~heartbeat[23]} : LED_FAIL;
      end
    end
  end
endmodule
