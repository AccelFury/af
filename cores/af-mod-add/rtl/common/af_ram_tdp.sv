// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_ram_tdp #(
  parameter int ADDR_WIDTH = 10,
  parameter int DATA_WIDTH = 32
) (
  input  logic                  i_clk_a,
  input  logic                  i_we_a,
  input  logic [ADDR_WIDTH-1:0] i_addr_a,
  input  logic [DATA_WIDTH-1:0] i_data_a,
  output logic [DATA_WIDTH-1:0] o_data_a,
  input  logic                  i_clk_b,
  input  logic                  i_we_b,
  input  logic [ADDR_WIDTH-1:0] i_addr_b,
  input  logic [DATA_WIDTH-1:0] i_data_b,
  output logic [DATA_WIDTH-1:0] o_data_b
);
  logic [DATA_WIDTH-1:0] mem [0:(1<<ADDR_WIDTH)-1];

  always_ff @(posedge i_clk_a) begin
    if (i_we_a) begin
      mem[i_addr_a] <= i_data_a;
    end
    o_data_a <= mem[i_addr_a];
  end

  always_ff @(posedge i_clk_b) begin
    if (i_we_b) begin
      mem[i_addr_b] <= i_data_b;
    end
    o_data_b <= mem[i_addr_b];
  end
endmodule
