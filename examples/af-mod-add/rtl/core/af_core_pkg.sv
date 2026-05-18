// SPDX-License-Identifier: CERN-OHL-S-2.0
package af_core_pkg;
  parameter int DEFAULT_FIELD_BITS = 64;

  typedef logic [DEFAULT_FIELD_BITS-1:0] field_t;
  typedef struct packed {
    logic valid;
    logic ready;
    field_t a;
    field_t b;
    field_t result;
  } af_stream_t;

  function automatic [DEFAULT_FIELD_BITS:0] wide_add(input field_t lhs, input field_t rhs);
    wide_add = {1'b0, lhs} + {1'b0, rhs};
  endfunction

  function automatic field_t mod_add(
    input field_t lhs,
    input field_t rhs,
    input logic [DEFAULT_FIELD_BITS-1:0] modulus
  );
    logic [DEFAULT_FIELD_BITS:0] sum_full;
    logic [DEFAULT_FIELD_BITS-1:0] delta;

    sum_full = wide_add(lhs, rhs);
    if (sum_full >= {1'b0, modulus}) begin
      delta = sum_full[DEFAULT_FIELD_BITS-1:0] - modulus;
      mod_add = delta;
    end else begin
      mod_add = sum_full[DEFAULT_FIELD_BITS-1:0];
    end
  endfunction
endpackage
