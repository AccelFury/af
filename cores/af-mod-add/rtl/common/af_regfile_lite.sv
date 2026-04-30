// SPDX-License-Identifier: CERN-OHL-S-2.0
module af_regfile_lite #(
  parameter int ADDR_WIDTH = 4,
  parameter int DATA_WIDTH = 32
) (
  input  logic                  i_clk,
  input  logic                  i_we,
  input  logic [ADDR_WIDTH-1:0] i_waddr,
  input  logic [DATA_WIDTH-1:0] i_wdata,
  input  logic [ADDR_WIDTH-1:0] i_raddr,
  output logic [DATA_WIDTH-1:0] o_rdata
);
  logic [DATA_WIDTH-1:0] regs [0:(1<<ADDR_WIDTH)-1];
  always_ff @(posedge i_clk) begin
    if (i_we) begin
      regs[i_waddr] <= i_wdata;
    end
    o_rdata <= regs[i_raddr];
  end
endmodule
