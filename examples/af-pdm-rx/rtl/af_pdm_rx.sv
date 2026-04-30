// SPDX-License-Identifier: Apache-2.0
// Handwritten raw PDM bit-group receiver. It does not implement PDM-to-PCM.
module af_pdm_rx #(
  parameter int unsigned CLK_DIV   = 16,
  parameter int unsigned WORD_BITS = 32
) (
  input  logic                 clk,
  input  logic                 rst_n,
  input  logic                 pdm_data_i,
  output logic                 pdm_clk_o,
  output logic [WORD_BITS-1:0] sample_word_o,
  output logic                 sample_valid_o,
  input  logic                 sample_ready_i
);
  logic [31:0]          clk_div_count;
  logic [31:0]          bit_count;
  logic [WORD_BITS-1:0] shift_word;

  wire stream_can_accept = !sample_valid_o || sample_ready_i;
  wire sample_tick = clk_div_count == (CLK_DIV - 1);

  always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) begin
      clk_div_count  <= '0;
      pdm_clk_o      <= 1'b0;
      bit_count      <= '0;
      shift_word     <= '0;
      sample_word_o  <= '0;
      sample_valid_o <= 1'b0;
    end else begin
      if (sample_valid_o && sample_ready_i) begin
        sample_valid_o <= 1'b0;
      end

      if (sample_tick) begin
        clk_div_count <= '0;
        pdm_clk_o <= ~pdm_clk_o;

        if (stream_can_accept) begin
          shift_word <= {shift_word[WORD_BITS-2:0], pdm_data_i};
          if (bit_count == (WORD_BITS - 1)) begin
            sample_word_o <= {shift_word[WORD_BITS-2:0], pdm_data_i};
            sample_valid_o <= 1'b1;
            bit_count <= '0;
          end else begin
            bit_count <= bit_count + 1'b1;
          end
        end
      end else begin
        clk_div_count <= clk_div_count + 1'b1;
      end
    end
  end
endmodule
