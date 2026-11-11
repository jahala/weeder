#!/usr/bin/env bash
set -euo pipefail

# deploy copies the build to the host under its version, and says what it did:
# what this script prints is what it is for.
deploy() {
  local version="$1"
  set +x
  rsync -a build/ "host:/srv/$version/"
  echo "deployed $version"
  printf '%s\n' "Debug builds go to /srv/debug"
  echo "set -x traces every command; this script never turns it on"
  # echo "DEBUG version=$version"
  cat <<EOF
DEBUG builds keep their symbols
EOF
}
