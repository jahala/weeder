# demo

Each worker gets a `meta.json` and a `stop.sh` under its state directory, and
the hook scripts land in `state/hooks/notify.sh`. `bind.sh` is written beside
them.

The gemini worker reads `GEMINI_MODEL`, a pane knows itself by `TMUX_PANE`, and
opencode reads `OPENCODE_CONFIG_DIR`.
