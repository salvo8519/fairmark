# Builds the musllinux wheel for Linux arm64 (runner ubuntu-24.04-arm; multi-platform index, linux/arm64 used).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM ghcr.io/rust-cross/rust-musl-cross:aarch64-musl@sha256:02e790506c5f15a533bc05b3af8719b16a8a2314480f6f0fdc0166432035d9b4
