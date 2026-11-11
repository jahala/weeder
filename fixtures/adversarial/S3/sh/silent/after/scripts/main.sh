#!/usr/bin/env bash
set -euxo pipefail

source "$(dirname "$0")/deploy.sh"
echo "DEBUG deploying $1"
deploy "$1"
