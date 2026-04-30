// SPDX-License-Identifier: Apache-2.0
module tb_af_pdm_rx;
  logic clk;
  logic rst_n;
  logic pdm_data;
  logic pdm_clk;
  logic sample_valid;
  logic sample_bit;

  af_pdm_rx #(
    .OVERSAMPLE(4)
  ) dut (
    .clk(clk),
    .rst_n(rst_n),
    .pdm_data(pdm_data),
    .pdm_clk(pdm_clk),
    .sample_valid(sample_valid),
    .sample_bit(sample_bit)
  );

  initial begin
    clk = 1'b0;
    rst_n = 1'b0;
    pdm_data = 1'b0;
  end
endmodule
