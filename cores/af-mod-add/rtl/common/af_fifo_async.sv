// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_fifo_async #(
  parameter int DATA_WIDTH = 64,
  parameter int DEPTH = 2
) (
  input  logic                  i_wr_clk,
  input  logic                  i_wr_rst_n,
  input  logic                  i_wr_en,
  input  logic [DATA_WIDTH-1:0] i_wr_data,
  output logic                  o_full,
  input  logic                  i_rd_clk,
  input  logic                  i_rd_rst_n,
  input  logic                  i_rd_en,
  output logic                  o_empty,
  output logic [DATA_WIDTH-1:0] o_rd_data
);
  localparam int PTR_W = (DEPTH <= 2) ? 1 : $clog2(DEPTH);
  localparam int COUNT_W = PTR_W + 1;

  logic [DATA_WIDTH-1:0] mem [0:DEPTH-1];
  logic [PTR_W:0] wr_ptr;
  logic [PTR_W:0] rd_ptr;

  logic [COUNT_W-1:0] wr_count;
  logic [COUNT_W-1:0] rd_count;

  assign o_full = (wr_count[COUNT_W-1:0] == DEPTH[COUNT_W-1:0]);
  assign o_empty = (rd_count == wr_count);

  always_ff @(posedge i_wr_clk or negedge i_wr_rst_n) begin
    if (!i_wr_rst_n) begin
      wr_ptr <= '0;
      wr_count <= '0;
    end else if (i_wr_en && !o_full) begin
      mem[wr_ptr[PTR_W-1:0]] <= i_wr_data;
      wr_ptr <= wr_ptr + 1'b1;
      wr_count <= wr_count + 1'b1;
    end
  end

  always_ff @(posedge i_rd_clk or negedge i_rd_rst_n) begin
    if (!i_rd_rst_n) begin
      rd_ptr <= '0;
      rd_count <= '0;
    end else if (i_rd_en && !o_empty) begin
      rd_ptr <= rd_ptr + 1'b1;
      rd_count <= rd_count + 1'b1;
    end
  end

  always_comb begin
    o_rd_data = mem[rd_ptr[PTR_W-1:0]];
  end
endmodule
