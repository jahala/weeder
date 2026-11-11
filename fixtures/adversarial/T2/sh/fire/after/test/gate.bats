#!/usr/bin/env bats

setup() {
  if [ -f gate.log ]; then
    rm gate.log
  fi
}

@test "the gate passes a clean input" {
  run ./gate.sh clean.txt
  [ "$status" -eq 0 ]
  assert_success
}

@test "the gate refuses an empty input" {
  run ./gate.sh ""
  [ "$status" -eq 1 ]
  test -s gate.log
}
