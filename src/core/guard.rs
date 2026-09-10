//! What a guard hook is before anything is written down: the shell bundle git
//! runs, the marker that says weeder wrote it, and the ref lines git feeds a
//! pre-push hook on stdin. The face does the writing and the reading; nothing
//! here touches a disk.

use std::path::Path;

use crate::core::shell;

use crate::core::glob;

/// The hooks guard installs. Each name is git's own and is also the `weeder guard`
/// subcommand its bundle calls, so whoever reads a hook can run by hand exactly
/// what git runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    PreCommit,
    CommitMsg,
    PrePush,
    PreRebase,
}

/// The line a bundle carries so weeder can tell a hook it wrote from one it did
/// not, and read back the binary that bundle names. It is public because it is
/// also how a measurement finds the day a repository installed guard.
pub const BINARY_MARKER: &str = "# weeder-guard-binary:";

impl Hook {
    /// In the order git runs them, so a person reading `install` or `status`
    /// reads the stages in the order a commit meets them.
    pub const ALL: [Hook; 4] = [
        Hook::PreCommit,
        Hook::CommitMsg,
        Hook::PrePush,
        Hook::PreRebase,
    ];

    /// The hook git calls a file by this name, or `None` for a file that is not
    /// one of them.
    pub fn named(name: &str) -> Option<Hook> {
        Hook::ALL.into_iter().find(|hook| hook.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Hook::PreCommit => "pre-commit",
            Hook::CommitMsg => "commit-msg",
            Hook::PrePush => "pre-push",
            Hook::PreRebase => "pre-rebase",
        }
    }

    /// The one line saying what this hook refuses, for `install` to print.
    pub fn refuses(self) -> &'static str {
        match self {
            Hook::PreCommit => "an index that carries a finding weeder blocks on",
            Hook::CommitMsg => {
                "an index that blocks, where the message allows none of what it blocks on"
            }
            Hook::PrePush => {
                "a pushed range that blocks, and a non-fast-forward to a protected branch"
            }
            Hook::PreRebase => "a rebase that would rewrite a protected branch",
        }
    }

    /// Whether git hands this hook the arguments naming a branch. Those are the
    /// hooks that need to know which branches are protected, and the only ones
    /// whose bundle passes anything through.
    fn judges_a_branch(self) -> bool {
        match self {
            Hook::PreCommit | Hook::CommitMsg => false,
            Hook::PrePush | Hook::PreRebase => true,
        }
    }

    /// Whether git hands this hook anything weeder has to be given. commit-msg
    /// is handed the file the message is being written in, and without it there
    /// is no message to read an allowance from; the branch hooks are handed the
    /// refs and the upstream they judge. pre-commit is handed nothing and asks
    /// git for the index itself.
    fn reads_its_arguments(self) -> bool {
        match self {
            Hook::PreCommit => false,
            Hook::CommitMsg | Hook::PrePush | Hook::PreRebase => true,
        }
    }
}

/// The bundle `install` writes for a hook: a POSIX shell script that finds the
/// weeder binary and hands git's own arguments to the matching `weeder guard`
/// subcommand. It names the binary by the absolute path weeder resolved at
/// install time and falls back to `PATH`, so a binary that moved is an error
/// git prints rather than a gate that quietly stops running.
pub fn script(hook: Hook, binary: &Path, protected: &[String]) -> String {
    let named = binary.display().to_string();
    let mut call = format!("exec \"$weeder\" guard {}", hook.name());
    if hook.judges_a_branch() {
        for branch in protected {
            call.push_str(&format!(" --protect {}", quote(branch)));
        }
    }
    if hook.reads_its_arguments() {
        call.push_str(" -- \"$@\"");
    }

    format!(
        "#!/bin/sh\n\
         # weeder guard, the law in git. `weeder guard install` wrote this file;\n\
         # `weeder guard uninstall` takes it away and puts back what was here.\n\
         {BINARY_MARKER} {named}\n\
         set -eu\n\
         \n\
         weeder={quoted}\n\
         [ -x \"$weeder\" ] || weeder=\"$(command -v weeder || true)\"\n\
         if [ ! -x \"$weeder\" ]; then\n\
         \tprintf '%s\\n' {complaint} >&2\n\
         \texit 1\n\
         fi\n\
         \n\
         {call}\n",
        quoted = quote(&named),
        complaint = quote(&format!(
            "weeder guard: the {} hook names the weeder binary at {named}, which is not there, and none is on PATH. run weeder guard install again.",
            hook.name()
        )),
    )
}

/// The binary a bundle names, or `None` when weeder did not write this file. It is
/// how `status` knows a hook is still weeder's and `uninstall` knows which files
/// are its own to take away.
pub fn binary_named(script: &str) -> Option<&str> {
    script
        .lines()
        .find_map(|line| line.strip_prefix(BINARY_MARKER))
        .map(str::trim)
}

/// The hook a path under `.githooks/` names, or `None` for any other path.
pub fn hook_named(path: &str) -> Option<Hook> {
    Hook::named(path.strip_prefix(".githooks/")?)
}

/// Whether a file invokes `weeder guard <hook>`: the subcommand's two words side
/// by side in a command, with the binary that runs them in front. It is how
/// weeder knows a stage is installed when somebody else planted it, since a
/// planter that renders hooks from a manifest carries no marker of weeder's, and
/// a stage git runs is a stage whoever wrote the file.
///
/// The reading is a shell's. A `#` opening a word opens a comment, so words in a
/// comment are not there at all; quotes hold a word together, so the subcommand
/// named inside a message the hook prints is one word and invokes nothing; and a
/// newline, a semicolon, an ampersand, a pipe or a bracket ends the command, so
/// two words on either side of one are not side by side.
pub fn invokes(hook: Hook, script: &str) -> bool {
    // The script is read the way the harness hook reads a command line, by
    // `shell::commands`: quoting taken off, comments and redirections left out,
    // one word list per command. A hook is the stage when one of its commands
    // runs the binary with `guard <hook>` after it, whatever path names the
    // binary, since the stem names it by its own path and never by a marker.
    shell::commands(script).iter().any(|words| {
        words.iter().enumerate().skip(1).any(|(at, word)| {
            word == "guard" && words.get(at + 1).is_some_and(|next| next == hook.name())
        })
    })
}

/// Whether a file is, byte for byte, the bundle weeder writes for this hook: the
/// binary its marker names and the branches its exec line protects, handed back
/// to [`script`], produce exactly this text. Adopting weeder is then a change C1
/// can tell from a hook someone rewrote, so it goes through the gate rather than
/// around it; one byte of difference, and the file is a guardrail edit again.
pub fn is_own_bundle(hook: Hook, text: &str) -> bool {
    let Some(binary) = binary_named(text) else {
        return false;
    };
    script(hook, Path::new(binary), &protected_in(text)) == text
}

/// The branches a bundle's exec line protects, read back out of its
/// `--protect '…'` words in the order they were written.
fn protected_in(text: &str) -> Vec<String> {
    let Some(line) = text
        .lines()
        .find(|line| line.trim_start().starts_with("exec \"$weeder\" guard"))
    else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("--protect '") {
        rest = &rest[at + "--protect '".len()..];
        let mut branch = String::new();
        loop {
            let Some(quote) = rest.find('\'') else {
                return found;
            };
            branch.push_str(&rest[..quote]);
            rest = &rest[quote + 1..];
            // A single quote inside the word is spelled '\'' by `quote`.
            if let Some(after) = rest.strip_prefix("\\''") {
                branch.push('\'');
                rest = after;
                continue;
            }
            break;
        }
        found.push(branch);
    }
    found
}

/// One line of what git feeds a pre-push hook on stdin: the ref being pushed,
/// the commit it points at, the ref on the remote, and the commit that ref
/// points at now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRef {
    pub local_ref: String,
    pub local_sha: String,
    pub remote_ref: String,
    pub remote_sha: String,
}

impl PushRef {
    /// The push takes the remote ref away: there is no local commit to send.
    pub fn deletes(&self) -> bool {
        is_absent(&self.local_sha)
    }

    /// The remote has no such ref yet, so there is nothing to fast-forward from.
    pub fn creates(&self) -> bool {
        is_absent(&self.remote_sha)
    }
}

/// The refs a push carries. A line weeder cannot read as four fields is not a ref
/// git wrote, and is left out rather than guessed at.
pub fn parse_push_refs(input: &str) -> Vec<PushRef> {
    input
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some(PushRef {
                local_ref: fields.next()?.to_string(),
                local_sha: fields.next()?.to_string(),
                remote_ref: fields.next()?.to_string(),
                remote_sha: fields.next()?.to_string(),
            })
        })
        .collect()
}

/// Whether a ref is one of the protected branches. An entry is a branch name or
/// a glob over one (`release/*`), written short or in full; the comparison is
/// made on short names, which is how a person names a branch.
pub fn protects(protected: &[String], reference: &str) -> bool {
    let branch = short_branch(reference);
    protected
        .iter()
        .any(|entry| glob::matches(short_branch(entry), branch))
}

/// `refs/heads/main` is the branch `main`. Anything that is not a branch ref is
/// left as it is, so a tag never reads as the branch of the same name.
pub fn short_branch(reference: &str) -> &str {
    reference.strip_prefix("refs/heads/").unwrap_or(reference)
}

/// git spells "there is no such object" as an id of nothing but zeros, in
/// whatever width this repository's hash has.
fn is_absent(object: &str) -> bool {
    !object.is_empty() && object.bytes().all(|byte| byte == b'0')
}

/// A value as one shell word: single quotes, and the one character single quotes
/// cannot carry spelled the way sh spells it.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
