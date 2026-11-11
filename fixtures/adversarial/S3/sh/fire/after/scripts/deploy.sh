#!/usr/bin/env bash
set -euxo pipefail

# deploy copies the build to the host under its version.
deploy() {
  local version="$1"
  set -x
  echo "DEBUG version=$version"
  rsync -a build/ "host:/srv/$version/"
  >&2 printf 'debug: %s\n' "$version"
  set -o xtrace
  echo "deployed $version"
}
