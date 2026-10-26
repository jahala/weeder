# demo

The pane driver is `adapters/tmux.ts`, and `spawnPane()` is the one thing it
exports. The fake agents are `test/fixtures/fake-{claude,codex}.sh`, and each
has a smoke directory in `test/smoke/{claude,codex}/`.
