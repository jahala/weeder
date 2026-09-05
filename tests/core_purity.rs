use std::fs;
use std::path::Path;

#[test]
fn core_does_not_import_io_process_git_or_outer_layers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core");
    let mut stack = vec![root];
    let forbidden = [
        "use std::fs",
        "std::fs::",
        "use std::process",
        "std::process::",
        "use std::env",
        "std::env::",
        "use git2",
        "git2::",
        "crate::seams",
        "crate::faces",
    ];

    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).expect("source directory should be readable") {
            let entry = entry.expect("source entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).expect("source file should be readable");
            for needle in forbidden {
                assert!(
                    !source.contains(needle),
                    "{} imports forbidden core dependency `{}`",
                    path.display(),
                    needle
                );
            }
        }
    }
}
