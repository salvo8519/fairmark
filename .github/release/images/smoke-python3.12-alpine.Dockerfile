# Smoke test of the musllinux wheels with Python 3.12 on Alpine Linux (multi-platform index).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM python:3.12-alpine@sha256:e064671ca452259bab50e8e2c7bd896e39e9a343bb06ca25234adf6fe18d0fc7
