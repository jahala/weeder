// A child session sets CHILD_SESSION before it starts.
const PASSED: string[] = ["HTTP_PROXY", "NO_PROXY"];

export const HOOK = `case "$1" in
  TERM_PROGRAM | TMUX_PANE) ;;
esac`;

export function passed(): string[] {
  return PASSED;
}

export function kind(event: string): string {
  switch (event) {
    case "ToolCall":
      return "tool";
    default:
      return "other";
  }
}
