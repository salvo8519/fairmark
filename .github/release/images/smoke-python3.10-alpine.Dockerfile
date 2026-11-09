# Smoke test of the musllinux wheels with Python 3.10 on Alpine Linux (multi-platform index).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM python:3.10-alpine@sha256:8ca9f9d0f4df2af674092e7e9d79ce7985e783cecfe070f8ea850801b6520bf7
