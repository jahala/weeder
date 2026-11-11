#!/usr/bin/env bats

setup() {
  if [ -f gate.log ]; then
    rm gate.log
  fi
}

@test "the gate passes a clean input" {
  run ./gate.sh clean.txt
  [ "$status" -eq 0 ]
  [[ "$output" == *"passed"* ]]
  assert_success
}

@test "the gate refuses an empty input" {
  run ./gate.sh ""
  [ "$status" -eq 1 ]
  assert_output --partial "empty"
  test -s gate.log
  refute_output --partial "passed"
}
