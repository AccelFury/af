// SPDX-License-Identifier: CERN-OHL-S-2.0
package tb_pkg;
  parameter int FIELD_BITS = 64;
  parameter logic [FIELD_BITS-1:0] MODULUS = 64'hFFFF_FFFF_0000_0001;

  function automatic logic [FIELD_BITS-1:0] goldilocks_add(
    input logic [FIELD_BITS-1:0] a,
    input logic [FIELD_BITS-1:0] b
  );
    logic [FIELD_BITS:0] sum;
    sum = {1'b0,a} + {1'b0,b};
    if (sum >= {1'b0,MODULUS}) begin
      goldilocks_add = sum[FIELD_BITS-1:0] - MODULUS;
    end else begin
      goldilocks_add = sum[FIELD_BITS-1:0];
    end
  endfunction
endpackage
