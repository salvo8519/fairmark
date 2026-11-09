#!/usr/bin/env bash
# Prints the pinned reference <image>:<tag>@sha256:<digest> of a container image of the
# Release workflow, read from the FROM line of .github/release/images/<name>.Dockerfile.
# Fails unless that file has exactly one FROM line pinned by digest.
set -euo pipefail
name="${1:?usage: pinned-image.sh <name>}"
file="$(dirname "$0")/images/$name.Dockerfile"
if [ ! -f "$file" ]; then
  echo "::error::no pinned image file $file" >&2
  exit 1
fi
ref="$(sed -n 's/^FROM[[:space:]]\{1,\}\([^[:space:]]\{1,\}\)[[:space:]]*$/\1/p' "$file")"
if ! [[ "$ref" =~ ^[^@[:space:]]+:[^@:/[:space:]]+@sha256:[0-9a-f]{64}$ ]]; then
  echo "::error file=$file::expected one line 'FROM <image>:<tag>@sha256:<digest>'" >&2
  exit 1
fi
printf '%s\n' "$ref"
