#!/usr/bin/env bats

@test "deploy names the version" {
  set -x
  run ./scripts/main.sh 1.4.0
  echo "DEBUG what the suite saw: $output"
  [ "$status" -eq 0 ]
}
