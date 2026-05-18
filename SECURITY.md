# Security Policy

This template does not claim production cryptographic security.

Security model and disclosures:

- This template is designed for design correctness and reproducibility, not
  certification.
- No secret material is included in source, vectors, or reports.
- Side-channel resistance is **not guaranteed** unless explicitly implemented
  and reviewed.
- Timing, power, and EM leakage analysis are required for any sensitive
  deployment.
- RNG-based vectors are deterministic and fully public.

Vulnerability handling:

- report privately to: `security@accelfury.org`
- do not discuss issues on public channels before triage
- include affected commit, hardware config, and reproduction steps
