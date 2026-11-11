#!/usr/bin/env bats

setup() {
  source ./scripts/client.sh
}

@test "send prints the payload" {
  run send hello
  [ "$output" = hello ]
}
