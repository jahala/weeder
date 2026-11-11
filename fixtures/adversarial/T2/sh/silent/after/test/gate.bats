#!/usr/bin/env bats

setup() {
  rm -f gate.log
}

@test "the gate passes a clean input" {
  run ./gate.sh clean.txt
  [ "$status" -eq 0 ]
  [[ "$output" == *"passed"* ]]
  assert_success
  refute_output --partial "empty"
}

@test "the gate refuses an empty input" {
  run ./gate.sh ""
  echo "[ the log is written before the gate answers ]"
  [ "$status" -eq 1 ]
  assert_output --partial "empty"
  test -s gate.log
}
