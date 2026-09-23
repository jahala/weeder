//! A person's own git settings change nothing weeder judges.
//!
//! weeder reads the diff git writes, and git writes it the way the global
//! configuration of whoever runs it says. `diff.noprefix` drops the `a/` and `b/`
//! a header names its paths with, and `diff.mnemonicPrefix` swaps them for
//! letters that say where each side came from; either one left every run of
//! `check` unable to read the diff at all, and every commit refused under the
//! guard. So weeder names the prefixes itself, and the log a change gets is the
//! same log whatever the person running it prefers.

mod common;

use common::{conflicted_parser, Repo};

/// Global settings a person may keep, each of which reshapes what `git diff`
/// writes when nobody overrides it.
const SETTINGS: [(&str, &str); 5] = [
    ("noprefix", "[diff]\n\tnoprefix = true\n"),
    ("mnemonicPrefix", "[diff]\n\tmnemonicPrefix = true\n"),
    (
        "srcPrefix and dstPrefix",
        "[diff]\n\tsrcPrefix = left/\n\tdstPrefix = right/\n",
    ),
    (
        "suppressBlankEmpty",
        "[diff]\n\tsuppressBlankEmpty = true\n",
    ),
    ("color", "[color]\n\tui = always\n\tdiff = always\n"),
];

/// A change with a finding that blocks, a renamed file, and a blank context
/// line, so a setting that reshapes any of those has something to reshape.
fn repository() -> Repo {
    let repo = Repo::init();
    repo.write(
        "src/format.ts",
        "export function format(text: string): string {\n  const trimmed = text.trim();\n\n  return trimmed;\n}\n",
    );
    repo.write("src/old_name.ts", "export const ready = true;\n");
    repo.commit("the state the change starts from");
    repo.write(
        "src/format.ts",
        "export function format(text: string): string {\n  const trimmed = text.trim();\n\n  return trimmed.toLowerCase();\n}\n",
    );
    repo.git(&["mv", "src/old_name.ts", "src/new_name.ts"]);
    repo.write("src/parser.ts", &conflicted_parser(None));
    repo.stage_all();
    repo
}

#[test]
fn a_global_git_setting_changes_nothing_weeder_judges() {
    let repo = repository();
    let plain = repo.weeder(&["check", "--format", "sarif"]);
    assert_eq!(
        plain.code, 2,
        "the conflict marker blocks: {}",
        plain.stderr
    );

    let desk = tempfile::TempDir::new().expect("a directory for the settings");
    for (name, settings) in SETTINGS {
        let global = desk
            .path()
            .join(format!("{}.gitconfig", name.replace(' ', "-")));
        std::fs::write(&global, settings).expect("the settings should be writable");
        let global = global.display().to_string();
        let changed = repo.git_with(&["diff", "--cached"], &[("GIT_CONFIG_GLOBAL", &global)]);
        let unchanged = repo.git(&["diff", "--cached"]);
        assert_ne!(
            changed, unchanged,
            "{name}: git has to write the diff differently under the setting, or the test proves nothing"
        );

        let run = repo.weeder_with(
            &["check", "--format", "sarif"],
            &[("GIT_CONFIG_GLOBAL", &global)],
        );

        assert_eq!(
            run.code, plain.code,
            "{name}: the same change leaves with the same code: {}",
            run.stderr
        );
        assert_eq!(
            run.stdout, plain.stdout,
            "{name}: and writes the same log, byte for byte"
        );
    }
}
