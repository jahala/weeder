# demo

Each worker gets a `meta.json` and a `stop.sh` under its state directory, and
the hook scripts land in `state/hooks/notify.sh`. `writeHooks()` in
`src/adapters/hooks.ts` writes them.

The gemini worker reads `GEMINI_MODEL`, and a pane knows itself by `TMUX_PANE`.
