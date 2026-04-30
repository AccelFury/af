// SPDX-License-Identifier: Apache-2.0
module simple_counter (
  input  logic       clk,
  input  logic       rst_n,
  output logic [7:0] count
);
  always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) begin
      count <= 8'h00;
    end else begin
      count <= count + 8'h01;
    end
  end
endmodule
