#!/usr/bin/env bash
# Evidence for authoring c2: the recall campaign reads its rules from the
# catalogue, and a check rule with no planter or no reader fails the suite.
#
# The test that holds this sits beside the planters in
# xtask/src/mutate/inject.rs, where it can see them, so it is a unit test of the
# xtask binary rather than a file under xtask/tests. This runs that one test by
# name and refuses unless it ran and passed: a filter that matches nothing passes
# with zero tests, and zero tests prove nothing.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

name="every_check_rule_in_the_catalogue_has_a_planter_and_a_reader"
out="$(cargo test --package xtask --bin xtask "$name" -- --exact "mutate::inject::tests::$name" 2>&1)" || {
  echo "$out" >&2
  echo "$name failed" >&2
  exit 1
}
echo "$out" | grep -q "test result: ok. 1 passed" || {
  echo "$out" >&2
  echo "$name did not run: the filter matched no test, so nothing was proven" >&2
  exit 1
}
echo "the recall campaign plants and reads a case for every check rule the catalogue holds"
