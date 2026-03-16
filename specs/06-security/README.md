# Security Design Overview

This document suite covers the complete security posture of the VRC Class Reunion backend. The system handles Discord user identity data (PII) and must resist common web application attacks while maintaining operational simplicity.

## Threat Surface Summary

| Surface | Risk Level | Primary Threats |
|---------|-----------|-----------------|
| OAuth2 Callback | High | CSRF, code injection, open redirect |
| Session Cookies | High | Session hijacking, fixation |
| Profile Bio (Markdown) | Medium | XSS via Markdown |
| System API Token | High | Token leakage, unauthorized sync |
| Role Escalation | High | Privilege escalation, IDOR |
| Rate Limiting | Medium | DoS, brute force |
| Database | Medium | SQL injection (mitigated by SQLx compile-time checks) |

## Documents

| Document | Contents |
|----------|----------|
| [threat-model.md](./threat-model.md) | STRIDE threat analysis per component |
| [authentication-design.md](./authentication-design.md) | Session lifecycle, token security |
| [authorization-design.md](./authorization-design.md) | Role hierarchy, permission matrix |
| [data-security.md](./data-security.md) | Encryption, PII handling, secrets |
| [application-security.md](./application-security.md) | Input validation, CSP, CSRF, headers |

## Security Principles

1. **Defense in Depth**: Multiple overlapping protections (validation at domain layer + HTTP layer + DB constraints)
2. **Least Privilege**: Type-State Authorization ensures compile-time role enforcement
3. **Fail Closed**: Unknown states default to deny (e.g., missing session → 401, unknown role → 403)
4. **Zero Trust Internal**: System API uses Bearer token auth even for internal traffic
5. **Secure by Default**: HttpOnly/Secure/SameSite cookies, HSTS, CSP headers
