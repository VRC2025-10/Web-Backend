# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please DO NOT open a public issue for security vulnerabilities.**

### Preferred: GitHub Private Vulnerability Reporting

1. Go to the [Security tab](https://github.com/VRC2025-10/Web-Backend/security) of this repository
2. Click **"Report a vulnerability"**
3. Fill in the details of your finding

### What to Include

- Description of the vulnerability
- Steps to reproduce (proof of concept if possible)
- Impact assessment (what can an attacker do?)
- Affected component(s) and version(s)
- Any suggested fix or mitigation

### Response Timeline

- **Acknowledgment**: Within 48 hours of report
- **Initial Assessment**: Within 1 week
- **Fix & Disclosure**: Coordinated with the reporter; typically within 30 days for critical issues

### Disclosure Policy

We follow **coordinated disclosure**:

1. Reporter notifies us privately
2. We confirm the issue and develop a fix
3. We release the fix and publish a security advisory
4. Reporter is credited (unless they prefer anonymity)

## Security Measures

This project implements multiple layers of security:

### Authentication & Authorization
- Discord OAuth2 with CSRF protection (state parameter)
- Server-side sessions with cryptographically random IDs
- Type-state authorization — role checks enforced at compile time
- Constant-time token comparison via `subtle` crate

### Input Validation & Sanitization
- All user input validated with strict regex patterns and length limits
- Markdown rendered to HTML with allowlist-based sanitization (ammonia)
- SQL injection prevented by compile-time verified parameterized queries (SQLx)

### Rate Limiting
- Per-IP rate limiting via lock-free token bucket algorithm (governor)
- Configurable limits per API layer

### Infrastructure
- Non-root container execution
- Security headers via Caddy (HSTS, X-Content-Type-Options, X-Frame-Options)
- TLS via rustls (no OpenSSL dependency)

### CI/CD Security
- Automated dependency auditing (cargo-audit, cargo-deny)
- Container image scanning
- Weekly security scanning pipeline
- Nightly extended testing (Miri for undefined behavior detection)

## Dependencies

We actively monitor and update dependencies:

- **Dependabot** configured for weekly Cargo and GitHub Actions updates
- **cargo-audit** runs in CI for known vulnerability detection
- **cargo-deny** enforces license compliance and advisory checks
