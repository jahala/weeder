// A child session sets CHILD_SESSION before it starts.
pub const PASSED: [&str; 2] = ["HTTP_PROXY", "NO_PROXY"];

pub const HOOK: &str = r#"case "$1" in
  TERM_PROGRAM | TMUX_PANE) ;;
esac"#;

pub fn kind(event: &str) -> &'static str {
    match event {
        "ToolCall" => "tool",
        _ => "other",
    }
}
