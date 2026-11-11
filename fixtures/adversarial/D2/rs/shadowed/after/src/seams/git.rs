use std::process::Command;

pub fn read(path: &str) -> String {
    let output = Command::new("git").args(["show", path]).output();
    output.map(|out| String::from_utf8_lossy(&out.stdout).into_owned()).unwrap_or_default()
}
