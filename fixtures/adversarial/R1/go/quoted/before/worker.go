package worker

// A child session sets CHILD_SESSION before it starts.
var passed = []string{"HTTP_PROXY", "NO_PROXY"}

// Hook is the shell the worker installs.
const Hook = `case "$1" in
  TERM_PROGRAM | TMUX_PANE) ;;
esac`

// Passed names the variables a worker passes through.
func Passed() []string {
	return passed
}

// Kind names what an event is.
func Kind(event string) string {
	switch event {
	case "ToolCall":
		return "tool"
	}
	return "other"
}
