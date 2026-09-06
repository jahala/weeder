use weeder::core::{parse_diff, ChangeKind, LineKind};

#[test]
fn parses_git_diff_files_hunks_and_line_numbers() {
    let diff = r#"diff --git a/src/add.rs b/src/add.rs
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/src/add.rs
@@ -0,0 +1,2 @@
+pub fn add() {}
+pub fn second() {}
diff --git a/src/delete.rs b/src/delete.rs
deleted file mode 100644
index 2222222..0000000
--- a/src/delete.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-pub fn gone() {}
-pub fn also_gone() {}
diff --git a/src/lib.rs b/src/lib.rs
index 3333333..4444444 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,5 @@
 fn a() {}
-fn b() {}
+fn b2() {}
 fn c() {}
 fn d() {}
 fn e() {}
@@ -20,3 +20,4 @@ fn later() {}
 fn x() {}
-fn y() {}
+fn y() {}
+fn z() {}
 fn q() {}
diff --git a/src/old.rs b/src/new.rs
similarity index 75%
rename from src/old.rs
rename to src/new.rs
index 5555555..6666666 100644
--- a/src/old.rs
+++ b/src/new.rs
@@ -1 +1 @@
-pub fn old() {}
+pub fn new() {}
diff --git a/assets/blob.bin b/assets/blob.bin
new file mode 100644
index 0000000..7777777
Binary files /dev/null and b/assets/blob.bin differ
diff --git a/script.sh b/script.sh
old mode 100644
new mode 100755
diff --git a/no_newline.txt b/no_newline.txt
index 8888888..9999999 100644
--- a/no_newline.txt
+++ b/no_newline.txt
@@ -1 +1 @@
-old
\ No newline at end of file
+new
\ No newline at end of file
"#;

    let files = parse_diff(diff).expect("git diff should parse");
    assert_eq!(files.len(), 7);

    assert_eq!(files[0].change, ChangeKind::Added);
    assert_eq!(files[0].old_path, None);
    assert_eq!(files[0].new_path.as_deref(), Some("src/add.rs"));
    assert_eq!(files[0].hunks[0].old_start, 0);
    assert_eq!(files[0].hunks[0].old_count, 0);
    assert_eq!(files[0].hunks[0].new_start, 1);
    assert_eq!(files[0].hunks[0].new_count, 2);
    assert_eq!(files[0].hunks[0].lines[0].kind, LineKind::Added);
    assert_eq!(files[0].hunks[0].lines[0].old_line, None);
    assert_eq!(files[0].hunks[0].lines[0].new_line, Some(1));

    assert_eq!(files[1].change, ChangeKind::Deleted);
    assert_eq!(files[1].old_path.as_deref(), Some("src/delete.rs"));
    assert_eq!(files[1].new_path, None);
    assert_eq!(files[1].hunks[0].lines[1].old_line, Some(2));
    assert_eq!(files[1].hunks[0].lines[1].new_line, None);

    assert_eq!(files[2].change, ChangeKind::Modified);
    assert_eq!(files[2].hunks.len(), 2);
    assert_eq!(files[2].hunks[0].lines[1].old_line, Some(2));
    assert_eq!(files[2].hunks[0].lines[2].new_line, Some(2));
    assert_eq!(files[2].hunks[1].lines[1].old_line, Some(21));
    assert_eq!(files[2].hunks[1].lines[2].new_line, Some(21));

    assert_eq!(files[3].change, ChangeKind::Renamed);
    assert_eq!(files[3].old_path.as_deref(), Some("src/old.rs"));
    assert_eq!(files[3].new_path.as_deref(), Some("src/new.rs"));
    assert_eq!(files[3].hunks[0].lines[0].kind, LineKind::Removed);
    assert_eq!(files[3].hunks[0].lines[1].kind, LineKind::Added);

    assert_eq!(files[4].change, ChangeKind::Binary);
    assert!(files[4].hunks.is_empty());

    assert_eq!(files[5].change, ChangeKind::ModeOnly);
    assert_eq!(files[5].old_mode.as_deref(), Some("100644"));
    assert_eq!(files[5].new_mode.as_deref(), Some("100755"));

    assert!(files[6].hunks[0].lines[0].no_newline);
    assert!(files[6].hunks[0].lines[1].no_newline);
}
