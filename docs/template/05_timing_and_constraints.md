# Timing and Constraints

Timings are collected under `boards/*/constraints/*`.

- Tang families: `.cst` files are draft, include basic clock pin assumptions
  only.
- Cyclone IV: `project.qsf` and `timing.sdc` are draft placeholders.

For real signoff:

1. run actual vendor implementation
2. close timing on worst-case corners
3. replace draft constraints with board-tested values
