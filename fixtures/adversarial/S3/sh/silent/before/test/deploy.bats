#!/usr/bin/env bats

@test "deploy names the version" {
  run ./scripts/main.sh 1.4.0
  [ "$status" -eq 0 ]
}
