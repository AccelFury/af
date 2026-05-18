# ADR 0004: Generated Vs Handwritten RTL

Handwritten RTL remains the source of hardware logic. Generated files may cover
wrappers, manifest export, build scripts, CI, and reports.

Generators must not silently create critical hardware logic such as CDC, FIFO,
audio filters, or bus bridges unless that logic comes from a separately
versioned and tested library.
