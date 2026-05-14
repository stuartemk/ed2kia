# Security Disclosure Policy - ed2kIA

## Responsible Disclosure

We take security vulnerabilities seriously and appreciate responsible disclosure from the security research community.

## Reporting a Vulnerability

### How to Report

1. **Email**: Send details to `security@ed2kIA.org`
2. **Encryption**: Use our PGP key (available on key servers)
3. **Response Time**: We will acknowledge within 48 hours
4. **Updates**: Weekly status updates during investigation

### Required Information

- **Title**: Clear description of the vulnerability
- **Type**: Classification (e.g., RCE, DoS, Information Disclosure)
- **Description**: Detailed technical explanation
- **Reproduction Steps**: Clear steps to reproduce
- **Impact**: Potential impact assessment
- **Remediation**: Suggested fix (if available)
- **CVE**: Reference (if already assigned)

### Example Report

```
Title: Remote Code Execution in WASM Sandbox Escape
Type: RCE (CWE-94)
CVSS: 9.8 (Critical)

Description:
The WASM sandbox in src/security/wasm_sandbox.rs fails to properly
validate memory boundaries when executing untrusted SAE modules,
allowing potential escape to host system.

Reproduction:
1. Compile malicious WASM module with memory overflow
2. Load via ed2kia wasm-load command
3. Trigger forward pass with crafted input
4. Observe host memory access

Impact:
- Full host compromise
- Private key theft
- Network consensus manipulation

Suggested Fix:
Add bounds checking in read_memory_from_caller() function
```

## Vulnerability Categories

### Critical (P1)

- Remote code execution (RCE)
- Private key compromise
- Consensus manipulation
- ZKP verification bypass

**Response**: 24 hours
**Fix Timeline**: 7 days

### High (P2)

- Denial of service (DoS)
- Information disclosure
- Reputation manipulation
- Lease hijacking

**Response**: 48 hours
**Fix Timeline**: 14 days

### Medium (P3)

- Local privilege escalation
- Resource exhaustion
- Configuration bypass

**Response**: 1 week
**Fix Timeline**: 30 days

### Low (P4)

- Minor information leaks
- Cosmetic issues
- Documentation gaps

**Response**: 2 weeks
**Fix Timeline**: Next release

## What NOT to Report

- Vulnerabilities in third-party dependencies (report to upstream)
- Theoretical attacks without proof of concept
- Social engineering (phishing, etc.)
- Availability issues (use issue tracker for bugs)
- Vulnerabilities requiring physical access

## Our Commitments

### We Will

- Acknowledge receipt within 48 hours
- Provide regular status updates
- Credit the reporter (unless anonymous requested)
- Work with you to understand the issue
- Fix the vulnerability in a timely manner
- Disclose the vulnerability publicly (with coordinator approval)

### We Will NOT

- Take legal action against reporters
- Silence or threaten reporters
- Delay fixes unnecessarily
- Disclose reporter identity without permission

## Embargo Policy

We request a **90-day embargo** from publication until fix is deployed, unless:

- Active exploitation is detected (immediate disclosure)
- No fix is available within 60 days (coordinated disclosure)
- Reporter requests earlier disclosure (with justification)

## Bug Bounty

Currently, ed2kIA operates a ** recognition-based** disclosure program. We plan to launch a monetary bug bounty program in Q2 2026.

### Recognition

- Credit in security advisory
- Special role in community channels
- Contribution to project security hall of fame
- Priority review of future contributions

## Security Best Practices

### For Node Operators

1. **Keep updated**: Run latest stable version
2. **Secure keys**: Store Ed25519 keys securely
3. **Firewall**: Restrict access to necessary ports only
4. **Monitor**: Enable health checks and alerting
5. **Backup**: Regular backups of reputation ledger

### For Developers

1. **Code review**: All PRs require 2 approvals
2. **Dependency updates**: Regular audits of Cargo.lock
3. **Testing**: Comprehensive test coverage
4. **Secrets**: Never commit secrets or credentials
5. **Documentation**: Document security considerations

## Security Contacts

| Role | Contact |
|------|---------|
| Security Lead | security@ed2kIA.org |
| PGP Key | Available on key servers |
| Emergency | security@ed2kIA.org (mark URGENT) |

## Past Security Advisories

| Date | CVE | Severity | Description | Status |
|------|-----|----------|-------------|--------|
| 2025-12-15 | N/A | Medium | WASM memory bounds check | Fixed v0.4.2 |
| 2025-11-20 | N/A | Low | Log injection in feedback | Fixed v0.4.1 |

## Legal

By submitting a vulnerability report, you agree to:

1. Not disclose the vulnerability publicly until we do
2. Not exploit the vulnerability for any purpose
3. Not damage any systems or data during testing
4. Work with us to reproduce and fix the issue

This policy is inspired by:
- [Chrome Security Reward Program](https://bugs.chromium.org/p/chromium/issues/detail?id=456)
- [RustSec Advisory Database](https://rustsec.org/)
- [OWASP Vulnerability Disclosure](https://owasp.org/www-project-vulnerability-disclosure-cheat-sheet/)
