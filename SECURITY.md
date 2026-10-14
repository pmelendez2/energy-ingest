<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Security

Do not commit:

- utility account identifiers;
- meter identifiers tied to real customers;
- raw private energy-usage exports;
- OAuth client secrets;
- access tokens;
- refresh tokens;
- passwords;
- API keys;
- decrypted SOPS/age secret files.

Static deployment credentials may be supplied at runtime through SOPS/age or
another secure secret-management mechanism.

Security issues should not be disclosed publicly until an appropriate private
reporting mechanism has been established.
