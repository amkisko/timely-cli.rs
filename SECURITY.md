# SECURITY

## Reporting a Vulnerability

Do not open a public GitHub issue for security vulnerabilities.

Email security details to: security@kiskolabs.com

Include: description, steps to reproduce, potential impact, and suggested fix
(if available).

Alternatively, report confidentially via GitHub: use the repository's Security
tab → Report a vulnerability, or open a
[private security advisory](https://github.com/amkisko/timely-cli.rs/security/advisories/new).

### Response Timeline

- We will acknowledge receipt of your report
- We will provide an initial assessment
- We will keep you informed of our progress and resolution timeline

### Disclosure Policy

- We will work with you to understand and resolve the issue
- We will credit you for the discovery (unless you prefer to remain anonymous)
- We will publish a security advisory after the vulnerability is patched
- We will coordinate public disclosure with you

## Automation Security

* Context Isolation: It is strictly forbidden to include production credentials,
  API keys, or Personally Identifiable Information (PII) in prompts sent to
  third-party LLMs or automation services.

* Supply Chain: All automated dependencies must be verified.

## Token handling (timely-cli)

Prefer a secret backend (1Password, Bitwarden, or KeePassXC) or a token file so
credentials stay out of shell history and process listings. Do not commit tokens,
OAuth client secrets, or refresh tokens. Avoid pasting live credentials into
issues, pull requests, or logs.

Home config and auth export paths may store secret-backend pointers or redacted
fingerprints. They must not store live bearer tokens or client secrets in shared
or public material.

## Supported versions

We release patches for the latest minor version. Security updates are prioritized
for the current stable release.
