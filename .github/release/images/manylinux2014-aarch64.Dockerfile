# Builds the manylinux wheel for Linux arm64 (runner ubuntu-24.04-arm, linux/arm64).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM quay.io/pypa/manylinux2014_aarch64:latest@sha256:101d2292840109c9d329fcfcd7b89b46f59e19ca23cbfdcf059fb389994665fc
