// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_xilinx_ram_tdp #(
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
  // draft wrapper placeholder for xilinx-specific RAM mapping
  af_ram_tdp #(
    .ADDR_WIDTH(ADDR_WIDTH),
    .DATA_WIDTH(DATA_WIDTH)
  ) u_fallback (
    .i_clk_a(i_clk_a),
    .i_we_a(i_we_a),
    .i_addr_a(i_addr_a),
    .i_data_a(i_data_a),
    .o_data_a(o_data_a),
    .i_clk_b(i_clk_b),
    .i_we_b(i_we_b),
    .i_addr_b(i_addr_b),
    .i_data_b(i_data_b),
    .o_data_b(o_data_b)
  );
endmodule
