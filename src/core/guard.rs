//! What a guard hook is before anything is written down: the shell bundle git
//! runs, the marker that says weed wrote it, and the ref lines git feeds a
//! pre-push hook on stdin. The face does the writing and the reading; nothing
//! here touches a disk.

use std::path::Path;

use crate::core::glob;

/// The hooks guard installs. Each name is git's own and is also the `weed guard`
/// subcommand its bundle calls, so whoever reads a hook can run by hand exactly
/// what git runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    PreCommit,
    PrePush,
    PreRebase,
}

/// The line a bundle carries so weed can tell a hook it wrote from one it did
/// not, and read back the binary that bundle names.
const BINARY_MARKER: &str = "# weed-guard-binary:";

impl Hook {
    pub const ALL: [Hook; 3] = [Hook::PreCommit, Hook::PrePush, Hook::PreRebase];

    pub fn name(self) -> &'static str {
        match self {
            Hook::PreCommit => "pre-commit",
            Hook::PrePush => "pre-push",
            Hook::PreRebase => "pre-rebase",
        }
    }

    /// The one line saying what this hook refuses, for `install` to print.
    pub fn refuses(self) -> &'static str {
        match self {
            Hook::PreCommit => "an index that carries a finding weed blocks on",
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
            Hook::PreCommit => false,
            Hook::PrePush | Hook::PreRebase => true,
        }
    }
}

/// The bundle `install` writes for a hook: a POSIX shell script that finds the
/// weed binary and hands git's own arguments to the matching `weed guard`
/// subcommand. It names the binary by the absolute path weed resolved at
/// install time and falls back to `PATH`, so a binary that moved is an error
/// git prints rather than a gate that quietly stops running.
pub fn script(hook: Hook, binary: &Path, protected: &[String]) -> String {
    let named = binary.display().to_string();
    let mut call = format!("exec \"$weed\" guard {}", hook.name());
    if hook.judges_a_branch() {
        for branch in protected {
            call.push_str(&format!(" --protect {}", quote(branch)));
        }
        call.push_str(" -- \"$@\"");
    }

    format!(
        "#!/bin/sh\n\
         # weed guard, the law in git. `weed guard install` wrote this file;\n\
         # `weed guard uninstall` takes it away and puts back what was here.\n\
         {BINARY_MARKER} {named}\n\
         set -eu\n\
         \n\
         weed={quoted}\n\
         [ -x \"$weed\" ] || weed=\"$(command -v weed || true)\"\n\
         if [ ! -x \"$weed\" ]; then\n\
         \tprintf '%s\\n' {complaint} >&2\n\
         \texit 1\n\
         fi\n\
         \n\
         {call}\n",
        quoted = quote(&named),
        complaint = quote(&format!(
            "weed guard: the {} hook names the weed binary at {named}, which is not there, and none is on PATH. run weed guard install again.",
            hook.name()
        )),
    )
}

/// The binary a bundle names, or `None` when weed did not write this file. It is
/// how `status` knows a hook is still weed's and `uninstall` knows which files
/// are its own to take away.
pub fn binary_named(script: &str) -> Option<&str> {
    script
        .lines()
        .find_map(|line| line.strip_prefix(BINARY_MARKER))
        .map(str::trim)
}

/// The hook a path under `.githooks/` names, or `None` for any other path.
pub fn hook_named(path: &str) -> Option<Hook> {
    let name = path.strip_prefix(".githooks/")?;
    Hook::ALL.into_iter().find(|hook| hook.name() == name)
}

/// Whether a file is, byte for byte, the bundle weed writes for this hook: the
/// binary its marker names and the branches its exec line protects, handed back
/// to [`script`], produce exactly this text. Adopting weed is then a change C1
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
        .find(|line| line.trim_start().starts_with("exec \"$weed\" guard"))
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

/// The refs a push carries. A line weed cannot read as four fields is not a ref
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
