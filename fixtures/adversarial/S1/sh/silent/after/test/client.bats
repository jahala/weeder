#!/usr/bin/env bats

setup() {
  source ./scripts/client.sh
}

@test "send prints the payload" {
  run send hello
  [ "$output" = hello ]
}

@test "label names a piece of work" {
  # TODO cover the done state too
  run label todo
  [ "$output" = "TODO: written by the caller" ]
}
