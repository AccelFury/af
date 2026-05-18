// SPDX-License-Identifier: CERN-OHL-S-2.0
interface af_stream_if #(parameter int FIELD_BITS = 64) (
  input  logic                  clk,
  input  logic                  rst_n
);
  logic                  valid;
  logic                  ready;
  logic [FIELD_BITS-1:0] a;
  logic [FIELD_BITS-1:0] b;
  logic [FIELD_BITS-1:0] result;

  modport master (
    input  ready,
    output valid,
    output a,
    output b,
    input  result
  );

  modport slave (
    output ready,
    input  valid,
    input  a,
    input  b,
    output result
  );
endinterface
