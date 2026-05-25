# Security Policy

`af` is a CLI/toolchain project. Production readiness for this repository does
not imply cryptographic certification, hardware security certification, timing
closure, CDC/RDC signoff, vendor bitstream signoff, or board hardware readiness.

## Reporting Vulnerabilities

Report security issues privately through GitHub private vulnerability reporting
/ GitHub Security Advisories for this repository.

Do not open a public issue for a vulnerability before maintainers have triaged
it. Include:

- affected commit SHA or release tag;
- exact command and `--json` output when applicable;
- affected generated core or manifest, if any;
- reproduction steps and impact.

## Supported Versions

| Version line             | Support status                                                 |
| ------------------------ | -------------------------------------------------------------- |
| `0.2.0-rc.x`             | Supported for release-candidate security fixes.                |
| `0.1.x`                  | Best-effort until 2026-08-23.                                  |
| Older / untagged commits | Not supported; reproduce on a supported tag or current `main`. |

## Deprecation Window

Security-relevant CLI/report/schema surfaces get a 90-day deprecation window
after a superseding release unless an actively exploited issue requires faster
removal. Breaking security changes are documented in `CHANGELOG.md`.

## Scope

In scope:

- CLI command execution, path handling, generated artifact safety, release
  provenance, and dependency/toolchain handling in this repository.

Out of scope unless separate evidence exists:

- side-channel resistance of generated or example RTL;
- FIPS/Common Criteria/safety certification claims;
- board-level electrical safety;
- vendor EDA installer or license vulnerabilities.
