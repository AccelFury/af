# Interface

`af_mod_add` uses a ready/valid stream contract.

Inputs:

- `i_clk`: clock
- `i_rst_n`: active-low synchronous reset (synchronized at top-level board
  wrappers)
- `i_valid`: input valid
- `o_ready`: core capacity output (can accept input)
- `i_a`: field operand A
- `i_b`: field operand B

Outputs:

- `o_valid`: output valid
- `i_ready`: downstream ready
- `o_result`: field result

Computation is:

- when `i_valid && o_ready`: accept one operand pair
- produce `result = (a + b) mod MODULUS`
- output remains stable while `i_ready == 0`
