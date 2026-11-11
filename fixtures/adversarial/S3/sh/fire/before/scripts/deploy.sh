#!/usr/bin/env bash
set -euo pipefail

# deploy copies the build to the host under its version.
deploy() {
  local version="$1"
  rsync -a build/ "host:/srv/$version/"
  echo "deployed $version"
}
