// SPDX-License-Identifier: Apache-2.0
module tb_af_pdm_rx;
  logic       clk;
  logic       rst_n;
  logic       pdm_data_i;
  logic       pdm_clk_o;
  logic [7:0] sample_word_o;
  logic       sample_valid_o;
  logic       sample_ready_i;

  af_pdm_rx #(
    .CLK_DIV(2),
    .WORD_BITS(8)
  ) dut (
    .clk(clk),
    .rst_n(rst_n),
    .pdm_data_i(pdm_data_i),
    .pdm_clk_o(pdm_clk_o),
    .sample_word_o(sample_word_o),
    .sample_valid_o(sample_valid_o),
    .sample_ready_i(sample_ready_i)
  );

  initial begin
    clk = 1'b0;
    forever #5 clk = ~clk;
  end

  initial begin
    rst_n = 1'b0;
    pdm_data_i = 1'b0;
    sample_ready_i = 1'b1;
    repeat (4) @(posedge clk);
    if (sample_valid_o !== 1'b0) $fatal(1, "sample_valid_o asserted during reset");

    rst_n = 1'b1;
    repeat (20) begin
      @(posedge clk);
      pdm_data_i = ~pdm_data_i;
    end

    sample_ready_i = 1'b0;
    repeat (8) @(posedge clk);
    sample_ready_i = 1'b1;
    repeat (8) @(posedge clk);
    $finish;
  end
endmodule
