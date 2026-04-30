// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_fifo_sync #(
  parameter int DATA_WIDTH = 64,
  parameter int DEPTH = 2
) (
  input  logic                  i_clk,
  input  logic                  i_rst_n,
  input  logic                  i_wr_en,
  input  logic [DATA_WIDTH-1:0] i_wr_data,
  output logic                  o_full,
  input  logic                  i_rd_en,
  output logic                  o_empty,
  output logic [DATA_WIDTH-1:0] o_rd_data
);
  localparam int ADDR_W = (DEPTH <= 2) ? 1 : $clog2(DEPTH);
  localparam int COUNT_W = (DEPTH <= 1) ? 1 : $clog2(DEPTH + 1);
  localparam logic [COUNT_W-1:0] DEPTH_COUNT = COUNT_W'(DEPTH);

  logic [DATA_WIDTH-1:0] mem [0:DEPTH-1];
  logic [ADDR_W:0] wr_ptr;
  logic [ADDR_W:0] rd_ptr;
  logic [COUNT_W-1:0] count;

  assign o_full  = (count == DEPTH_COUNT);
  assign o_empty = (count == 0);
  assign o_rd_data = mem[rd_ptr[ADDR_W-1:0]];

  always_ff @(posedge i_clk or negedge i_rst_n) begin
    if (!i_rst_n) begin
      wr_ptr <= '0;
      rd_ptr <= '0;
      count <= '0;
    end else begin
      if (i_wr_en && !o_full) begin
        mem[wr_ptr[ADDR_W-1:0]] <= i_wr_data;
        wr_ptr <= wr_ptr + 1'b1;
        count <= count + 1'b1;
      end
      if (i_rd_en && !o_empty) begin
        rd_ptr <= rd_ptr + 1'b1;
        count <= count - 1'b1;
      end
      if ((i_wr_en && !o_full) && (i_rd_en && !o_empty)) begin
        count <= count;
      end
    end
  end
endmodule
