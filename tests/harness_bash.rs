//! Every test that runs a shell script runs it through `common::bash`.
//!
//! On Windows a bare `bash` is System32's launcher for the Windows Subsystem
//! for Linux, which runs nothing on the runner. Two test files called it, and
//! master was red on Windows for three pushes before anyone saw it. The harness
//! names the bash Git for Windows ships, and this holds every test file to it.

use std::path::Path;

#[test]
fn no_test_file_starts_a_bare_bash() {
    // Written in two halves, so this file does not match itself.
    let bare = concat!("Command::new(", "\"bash\")");
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&tests).expect("the tests folder should be readable") {
        let path = entry.expect("a tests entry should be readable").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).expect("a test file should be readable");
            if text.contains(bare) {
                offenders.push(
                    path.file_name()
                        .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
                );
            }
        }
    }
    offenders.sort();
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "these test files start a bare bash, which is WSL's launcher on Windows; run the script with common::bash"
    );
}
