# Smoke test of the musllinux wheels with Python 3.15 on Alpine Linux (multi-platform index).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM python:3.15-alpine@sha256:a0f2e268f2ce8f3709611a90054ec6a8ae31f6251f5896b8648b9bb99835e30a
