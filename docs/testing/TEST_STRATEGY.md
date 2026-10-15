<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Test Strategy

**Purpose:** Describe the project-wide testing approach and boundaries.

**Intended Audience:** Developers, test authors, and maintainers.

Testing is part of the architecture of this project.

The project will use:

- unit tests;
- parser regression tests;
- integration tests;
- persistence tests;
- provider-adapter tests;
- OAuth tests using mocks or test providers;
- negative/error-path tests;
- idempotency and correction tests;
- security and secret-scanning checks.

Tests should be repeatable both locally and in CI.

Real private utility data must not be committed to the public repository.
Public regression fixtures should be synthetic or independently reviewed for
privacy.
