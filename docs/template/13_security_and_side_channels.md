# Security and Side-Channel Policy

## Security posture

- This template does not claim cryptographic production security.
- The repository templates are for engineering and verification workflows.
- Publicly available vectors are deterministic and non-secret.

## Side-channel and leakage notes

- no constant-time guarantees unless explicitly documented in an IP
  implementation note
- no formal side-channel hardening is provided by default
- timing, power, and EM leakage analysis are required before secure deployment
- do not reuse deterministic vectors as secret inputs

## Operational limits

- random/public test vectors are suitable for CI and regression
- do not encode private keys or device secrets in source or artifacts
- review pin and reset behavior before field deployment

## Responsible disclosure

Report issues privately to `security@accelfury.org` and do not publish
impact-sensitive details before triage.
