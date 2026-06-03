# O4 Safe Provenance Scrub

This slice performs the narrow provenance cleanup that is safe before a wider crate/command rename.

## Applied

- `OpenAI Codex concept` in donor-domain conceptual source list was replaced with neutral `external agent concept`.
- `OpenAiEmployee` feedback audience enum/usage was renamed to `InternalTester`.
- Third-party attribution was moved to `THIRD_PARTY_NOTICES.md`.

## Not applied

- No blanket `OpenAI` rename was performed because provider/API identifiers may be functional.
- No third-party license headers were removed.
- No crate or command rename was attempted.
