# Security Policy

## Supported versions

Pre-release: best-effort fixes on `main` only.

## Reporting

- Do not open public issues for updater/signature verification or save-handling exploits.
- Contact maintainers privately (GitHub Security Advisories preferred).
- Expect acknowledgement within 72h.

## Update integrity

- DB feeds must be ed25519-signed. Unsigned manifests are rejected by the client.
- Report any signature-bypass or silent-update behaviour as critical.
