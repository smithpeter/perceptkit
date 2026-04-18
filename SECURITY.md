# Security Policy

## Supported Versions

v0.1.x is the only supported version during alpha/beta.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | ✅        |
| < 0.1   | ❌        |

## Signal Model Commitment

perceptkit's "Signal Model" (DATA.md §2) is a security-level promise:

> The `perceptkit-core` binary has zero network calls.

If you observe network traffic attributable to `perceptkit-core` (not your application, not a Reflector backend you configured), this is a **security bug**. Please report immediately.

## Reporting a Vulnerability

Email: **zouyongming@gmail.com**

Please do not open a public GitHub issue for security bugs.

Include:
- Affected version(s)
- Reproduction steps
- Impact assessment
- Suggested fix (if any)

## Response SLA

- Acknowledgement: within 72 hours
- Initial assessment: within 1 week
- Fix target: 30 days for high severity, best-effort for lower

## Supply Chain

- Dependencies audited via `cargo deny` in CI
- `cargo audit` recommended weekly (not yet automated — v0.1.1)
- No external network calls means minimal attack surface in core

## Data Handling

- perceptkit does not collect user data (Signal Model)
- No telemetry, ever, even opt-in (STRATEGY §11.3)
- Contributors to the dataset sign DCO; data is CC0/CC-BY-NC per DATA.md §3.9
