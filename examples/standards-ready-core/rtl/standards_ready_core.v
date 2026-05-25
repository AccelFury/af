// SPDX-License-Identifier: Apache-2.0
`default_nettype none

module standards_ready_core #(
  parameter DATA_WIDTH = 8
) (
  input  wire                  clk,
  input  wire                  rst,
  input  wire                  s_valid,
  output wire                  s_ready,
  input  wire [DATA_WIDTH-1:0] s_data,
  output reg                   m_valid,
  input  wire                  m_ready,
  output reg  [DATA_WIDTH-1:0] m_data,
  input  wire                  cfg_enable,
  output wire                  status_busy,
  output reg  [7:0]            counter_words
);

  assign s_ready = cfg_enable && (!m_valid || m_ready);
  assign status_busy = m_valid && !m_ready;

  always @(posedge clk) begin
    if (rst) begin
      m_valid <= 1'b0;
      m_data <= {DATA_WIDTH{1'b0}};
      counter_words <= 8'h00;
    end else if (s_ready) begin
      m_valid <= s_valid;
      if (s_valid) begin
        m_data <= s_data;
        counter_words <= counter_words + 8'h01;
      end
    end else if (m_ready) begin
      m_valid <= 1'b0;
    end
  end

endmodule

`default_nettype wire
