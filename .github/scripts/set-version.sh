#!/usr/bin/env bash
# Writes the release tag's version (v1.2.3 -> 1.2.3) into the addon .toc,
# companion/Cargo.toml and companion/Cargo.lock. Does nothing without a tag.
# Uses perl because sed -i differs between GNU and macOS.
set -euo pipefail
cd "$(dirname "$0")/../.."

tag="${TAG:-}"
[ -n "$tag" ] || { echo "No release tag, keeping the versions in the repo."; exit 0; }

version="${tag#v}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Release tag '$tag' is not a version like v1.2.3 or v1.2.3-beta.1"; exit 1
fi

export VERSION="$version"
perl -pi -e 's/^## Version:.*/## Version: $ENV{VERSION}/' addon/Translate/Translate.toc
# Only the first version line in Cargo.toml is the package's own.
perl -pi -e 's/^version = ".*"/version = "$ENV{VERSION}"/ && ($done = 1) unless $done' companion/Cargo.toml
# In Cargo.lock, the version line right after our package's name.
perl -0pi -e 's/(name = "translate-companion"\nversion = )"[^"]*"/$1"$ENV{VERSION}"/' companion/Cargo.lock

grep -n '^## Version' addon/Translate/Translate.toc
grep -n -m1 '^version' companion/Cargo.toml
grep -n -A1 'name = "translate-companion"' companion/Cargo.lock
