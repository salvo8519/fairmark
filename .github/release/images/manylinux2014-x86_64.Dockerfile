# Builds the manylinux wheel for Linux x86_64 (runner ubuntu-latest, linux/amd64).
# Not built: the Release workflow reads this FROM line (.github/release/pinned-image.sh), and
# Dependabot updates the digest (.github/dependabot.yml, docs/en/project/dependencies.md).
FROM quay.io/pypa/manylinux2014_x86_64:latest@sha256:62eeff30ef134a85ec0f97109e008a319b0315ca5e8a6d754ee771556c02622a
