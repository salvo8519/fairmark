# Builds the musllinux wheel for Linux x86_64 (runner ubuntu-latest; multi-platform index, linux/amd64 used).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM ghcr.io/rust-cross/rust-musl-cross:x86_64-musl@sha256:3f1010eb48311dfde5d51f011163451940af34bd317355c4a8652ce6b4b61051
