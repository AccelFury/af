// SPDX-License-Identifier: Apache-2.0
// Minimal PDM bitstream receiver example. It does not implement PDM-to-PCM.
module af_pdm_rx #(
  parameter int unsigned OVERSAMPLE = 64
) (
  input  logic clk,
  input  logic rst_n,
  input  logic pdm_data,
  output logic pdm_clk,
  output logic sample_valid,
  output logic sample_bit
);
  logic [31:0] sample_count;

  assign pdm_clk = clk;

  always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) begin
      sample_count <= '0;
      sample_valid <= 1'b0;
      sample_bit <= 1'b0;
    end else begin
      sample_valid <= 1'b0;
      if (sample_count == (OVERSAMPLE - 1)) begin
        sample_count <= '0;
        sample_valid <= 1'b1;
        sample_bit <= pdm_data;
      end else begin
        sample_count <= sample_count + 1'b1;
      end
    end
  end
endmodule
