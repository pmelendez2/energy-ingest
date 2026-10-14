# Local API client credentials

The local application credential lifecycle follows ADR-0024.
Credentials are separate from database passwords and utility OAuth tokens.

The first Stage 2 implementation adds token administration and persistence.
It **does not yet start an HTTP API** or publish network ports.

Use the existing production Compose maintenance profile with
`api-token create NAME`, `api-token list`, or
`api-token revoke CLIENT_ID` through its `admin` service.
For example, when using the existing production environment:

```bash
# Run from deploy/docker-compose after normal production setup.
docker compose -f compose.prod.yml --profile maintenance \
  run --rm admin api-token list
```

`create` prints the plaintext token **once**, to stdout. Do not run
creation with shell tracing, terminal recording, or output logging.
Store the token using the project's secret-management policy.
Never include it in source control or diagnostic output.

`list` displays client IDs, names, and revoked/active status, never
verifiers or plaintext tokens. `revoke` invalidates one client token.
All credentials have equal normal application permissions initially.

Test using the project's isolated Docker test runner, including the
`api_client_db` integration test. No real customer records are needed.
