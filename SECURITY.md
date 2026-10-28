# Security Policy

## Supported Versions

Sayn is in early development. Security fixes will be applied to the latest version on the `main` branch.

| Version | Supported |
|---------|-----------|
| main    | ✅ Active development |

## Reporting a Vulnerability

If you discover a security vulnerability in Sayn, please report it responsibly.

**Preferred method:** Use [GitHub's private vulnerability reporting](https://github.com/kingjethro999/sayn/security/advisories/new) to submit a report.

**Please do not:**
- Open a public issue for security vulnerabilities
- Include real credentials, API keys, or tokens in reports
- Test vulnerabilities against production systems you do not own

## Security Scope

Sayn processes untrusted natural-language input and generates executable code. The following areas are particularly security-sensitive:

- **Path confinement** — Generated files must stay within the configured output root
- **Credential handling** — API keys and tokens must never be logged, committed, or exposed
- **AI output validation** — Model responses must be validated before filesystem writes
- **Dependency integrity** — Dependencies must come from trusted sources
- **Permission enforcement** — Privileged operations require explicit user authorization

## Acknowledgments

We appreciate responsible disclosure and will credit reporters (with permission) in release notes.
