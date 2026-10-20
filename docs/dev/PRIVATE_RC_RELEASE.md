<!-- SPDX-License-Identifier: MIT -->
<!-- AI-assisted: ChatGPT / GPT-6; 2026-10-20. -->

# Private Release Candidate Publication

## Purpose

Document maintainer-only release-candidate publishing to the private
homelab registry. This does not replace the official stable release process.

**Intended Audience:** Maintainers and developers.

## Preconditions

- Merge reviewed changes through a protected pull request and required CI.
- Synchronize a clean local `main` with `origin/main`.
- Use a Docker Buildx builder with `linux/amd64` and `linux/arm64` support.
- Configure Docker for the registry endpoint and authenticate using
  `docker login` before publication.
- Keep the private registry restricted to trusted LAN clients. It currently
  uses HTTP with Basic authentication, so HTTPS is recommended long term.

## Local verification

Run on the development host:

```bash
bash scripts/test-prod-release-versions.sh
bash scripts/test-private-rc-registry.sh
bash deploy/docker-compose/test.sh
```

The registry safety tests mock HTTP responses and do not publish images.
They permit publication only when an authenticated manifest lookup yields
HTTP 404. Existing tags, unauthorized requests, server errors, and network
errors must all stop publication.

## Publishing

After the feature branch is merged and local `main` is synchronized:

```bash
bash scripts/publish-private-rc.sh 0.1.5-rc.1
```

The version is an explicit argument. The registry defaults to
`tecoreg.tecocllc.home.arpa:5001`, and the image repository defaults to
`energy-ingest`. Maintainers may configure `ENGY_RC_REGISTRY` and
`ENGY_RC_REPOSITORY` when required; both values are validated.

The publisher prompts for registry credentials to perform a separate
manifest existence check before building or pushing. Do not put passwords
in shell history, repository files, or CI logs. A successful preflight
reduces accidental overwrites but does not provide atomic tag creation:
another publisher could race the check.

The generated installer ZIP includes the corresponding image reference.
The stable Docker Hub and GitHub release workflow remains unchanged.

## Deployment and rollback

Do not modify the running production installation until the candidate image,
package, and architecture manifests pass inspection. A private RC is not a
stable public release. Preserve unrelated Docker services, production HA,
MQTT, and original utility XML. Backup persistent data before any
explicitly authorized destructive reinstall.
