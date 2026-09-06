# dogfood — tilth-core

Notes from the worker on the tilth-core loop, 2026-09-05.

## tend2

- `tend2 verify … --force --runner 'bash scripts/check/run.sh {evidence}'` was the whole gate and needed no coaxing: three checks, three evidence paths, one command. Re-running it is cheap, which is what made it the thing to run first rather than last.
- The verifier re-runs every check under `--force`, c1 included. c1 clones tilth and runs its CI trio, so the gate is only fast the second time; the first run on a cold machine pays for a full clone and workspace build. Worth knowing before reading the silence as a hang.
- `tend2 verify` writes the stamps and the loop file is otherwise read-only for the worker. That split held cleanly here, nothing tempted a hand-flipped checkbox, because running the gate is strictly less work than editing the file.

## Where the map earned its keep

The loop's `## Tried` carried almaty's six surface notes from the channel, and every one of them was load-bearing: `test_entries` empty for three of the four languages, `#[test]` invisible through `OutlineEntry`, `TestEntry.callee`, the canonical scope `analyze_deps` wants. Reading them first turned what would have been three rounds of surprise into one design. A plan file would have gone stale; the loop did not.

## What the seam cost

The one thing the map did not predict: Python's `from x import y` produces no outline entry, so an import reader built on outline entries alone silently misses most Python imports. Found by probing the crate against real fixtures before writing assertions, which is the only reason the test asserts what tilth does rather than what the API doc hoped it did.
