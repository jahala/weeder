# demo

A worker passes `HTTP_PROXY` and `NO_PROXY` through, reports a `ToolCall`
event as a tool, and its hook leaves `TERM_PROGRAM` alone. A child session
sets `CHILD_SESSION` first.
