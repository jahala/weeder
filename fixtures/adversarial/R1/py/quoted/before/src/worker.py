# A child session sets CHILD_SESSION before it starts.
PASSED = ["HTTP_PROXY", "NO_PROXY"]

HOOK = """case "$1" in
  TERM_PROGRAM | TMUX_PANE) ;;
esac"""


def kind(event):
    match event:
        case "ToolCall":
            return "tool"
        case _:
            return "other"
