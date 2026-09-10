//! `weeder guard`, weeder's judgement put where a harness cannot route around it.
//!
//! `install` writes four POSIX shell hooks that call this binary and points
//! `core.hooksPath` at the directory holding them, keeping whatever that setting
//! was so `uninstall` can put it back. `status` says whether each hook is still
//! live. The four hook subcommands are what those scripts run: git calls them,
//! and the code they leave with is git's answer, so a hook that could not judge
//! stops the operation rather than waving it through.
//!
//! The stages are not interchangeable. pre-commit sees the index and no message,
//! so it names the allowance a person could write and leaves the verdict to
//! commit-msg, which sees the message and is the deciding stage for anything a
//! trailer may allow. pre-push sees the commits themselves, messages and all.

use std::path::{Path, PathBuf};

use crate::core::finding::{Finding, Level};
use crate::core::guard::{self, Hook, PushRef};
use crate::core::sarif::{EXIT_BLOCKED, EXIT_CLEAN, EXIT_COULD_NOT_RUN};
use crate::core::suppress::TRAILER;
use crate::faces::{check, read_config, Answer};
use crate::seams::{fs, git};

/// Where git is told to look, unless `--hooks-dir` names somewhere else. It sits
/// in the working tree, so a clone can install the same hooks without inventing
/// its own path.
const DEFAULT_HOOKS_DIR: &str = ".githooks";
/// The setting that decides which directory git runs hooks from. It is the whole
/// mechanism: one directory to install into, one setting for `status` to check.
const HOOKS_PATH_KEY: &str = "core.hooksPath";
/// Where guard keeps what it must put back. It lives in the git directory, so it
/// never reaches a commit and never travels with a clone.
const RECORD_DIR: &str = "weeder";
const PREVIOUS_HOOKS_PATH: &str = "previous-hooks-path";
const INSTALLED_HOOKS: &str = "installed-hooks";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weeder was called from; the repository is found from here.
    pub cwd: PathBuf,
    pub version: String,
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Install(Install),
    Status,
    Uninstall,
    PreCommit,
    CommitMsg(CommitMsg),
    PrePush(PrePush),
    PreRebase(PreRebase),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMsg {
    /// The file git is having the message written in, which git hands the hook
    /// as its one argument.
    pub message_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    /// Where the hooks are written, relative to the repository root unless it is
    /// an absolute path.
    pub hooks_dir: Option<PathBuf>,
    /// The branches the installed hooks protect. Empty leaves them reading
    /// `weeder.toml` at the moment git runs them.
    pub protect: Vec<String>,
    /// The binary the hooks will name, resolved by the caller: weeder's own path.
    pub binary: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrePush {
    pub protect: Vec<String>,
    /// What git puts after the hook's name: the remote, then its URL.
    pub arguments: Vec<String>,
    /// What git writes on the hook's stdin, one line per ref being pushed.
    pub refs: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreRebase {
    pub protect: Vec<String>,
    /// What git puts after the hook's name: the upstream, and the branch being
    /// rebased where the caller named one.
    pub arguments: Vec<String>,
}

pub fn run(request: &Request) -> Answer {
    match judge(request) {
        Ok(answer) => answer,
        // A guard that could not do its work must never read as a guard that
        // allowed the change, so every failure leaves with exit 3 and says why.
        Err(reason) => Answer {
            code: EXIT_COULD_NOT_RUN,
            stdout: String::new(),
            stderr: vec![reason],
        },
    }
}

fn judge(request: &Request) -> Result<Answer, String> {
    let root = git::repository_root(&request.cwd).map_err(|error| error.to_string())?;
    match &request.command {
        Command::Install(install) => self::install(&root, install),
        Command::Status => status(&root),
        Command::Uninstall => uninstall(&root),
        Command::PreCommit => pre_commit(&root, &request.version),
        Command::CommitMsg(message) => commit_msg(&root, &request.version, message),
        Command::PrePush(push) => pre_push(&root, &request.version, push),
        Command::PreRebase(rebase) => pre_rebase(&root, rebase),
    }
}

fn install(root: &Path, install: &Install) -> Result<Answer, String> {
    let binary = install.binary.display().to_string();
    if binary.contains('\n') {
        return Err(format!(
            "weeder is running from a path that carries a newline ({binary}), and a hook names its binary on one line of a script. move the binary somewhere a line can hold."
        ));
    }

    let named = install
        .hooks_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOOKS_DIR));
    let named = named.display().to_string();
    let directory = root.join(&named);
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    let mut written = Vec::new();
    for hook in Hook::ALL {
        let path = directory.join(hook.name());
        let script = guard::script(hook, &install.binary, &install.protect);
        fs::write(&path, &script).map_err(|error| error.to_string())?;
        fs::make_executable(&path).map_err(|error| error.to_string())?;
        written.push(format!("{named}/{}", hook.name()));
    }

    let records = records(root)?;
    fs::create_dir_all(&records).map_err(|error| error.to_string())?;
    let previous = records.join(PREVIOUS_HOOKS_PATH);
    // A second install must not overwrite the record with guard's own directory,
    // or the setting the repository started with is lost for good.
    if !fs::exists(&previous) {
        let setting = git::config_get(root, HOOKS_PATH_KEY).map_err(|error| error.to_string())?;
        fs::write(&previous, &setting.unwrap_or_default()).map_err(|error| error.to_string())?;
    }
    fs::write(
        &records.join(INSTALLED_HOOKS),
        &format!("{}\n", written.join("\n")),
    )
    .map_err(|error| error.to_string())?;
    git::config_set(root, HOOKS_PATH_KEY, &named).map_err(|error| error.to_string())?;

    let mut stdout = format!("weeder guard is installed: git runs its hooks from {named}.\n");
    for hook in Hook::ALL {
        stdout.push_str(&format!(
            "{:<10}  refuses {}\n",
            hook.name(),
            hook.refuses()
        ));
    }
    stdout.push_str(&protection(root, &install.protect)?);
    Ok(Answer {
        code: EXIT_CLEAN,
        stdout,
        stderr: Vec::new(),
    })
}

fn status(root: &Path) -> Result<Answer, String> {
    let Some(installed) = installed(root)? else {
        return Ok(missing(
            "weeder guard has installed no hooks in this repository, so git runs whatever it finds. run weeder guard install.",
        ));
    };

    let mut live = Vec::new();
    let mut misses = Vec::new();
    for entry in &installed {
        let path = root.join(entry);
        let Some(script) = fs::read_if_present(&path).map_err(|error| error.to_string())? else {
            misses.push(format!(
                "{entry} is missing, so git has no weeder hook to run there. run weeder guard install."
            ));
            continue;
        };
        let Some(binary) = guard::binary_named(&script) else {
            misses.push(format!(
                "{entry} is a file weeder did not write, so what git runs there is not weeder's judgement."
            ));
            continue;
        };
        if !fs::is_executable(&path) {
            misses.push(format!(
                "{entry} is not executable, and git walks past a hook it cannot run without a word."
            ));
            continue;
        }
        if !fs::is_executable(Path::new(binary)) {
            misses.push(format!(
                "{entry} names the weeder binary at {binary}, which is not there to run. run weeder guard install again."
            ));
            continue;
        }
        live.push(format!("{entry:<24}  live  {binary}"));
    }

    let directory = directory_of(&installed);
    match git::config_get(root, HOOKS_PATH_KEY).map_err(|error| error.to_string())? {
        None => misses.push(format!(
            "{HOOKS_PATH_KEY} is not set, so git runs its own hooks and never reaches weeder's."
        )),
        Some(setting) if !same_directory(root, &setting, directory) => misses.push(format!(
            "{HOOKS_PATH_KEY} points at {setting}, not at {directory}, so git runs hooks weeder did not write."
        )),
        Some(setting) => live.push(format!("{HOOKS_PATH_KEY:<24}  live  {setting}")),
    }

    let mut stdout = String::new();
    for line in live {
        stdout.push_str(&line);
        stdout.push('\n');
    }
    for miss in &misses {
        stdout.push_str(miss);
        stdout.push('\n');
    }
    if misses.is_empty() {
        stdout.push_str("weeder guard is live.\n");
        return Ok(Answer {
            code: EXIT_CLEAN,
            stdout,
            stderr: Vec::new(),
        });
    }
    stdout.push_str(&format!(
        "weeder guard is not the law here: {}.\n",
        counted(misses.len())
    ));
    Ok(Answer {
        code: EXIT_BLOCKED,
        stdout,
        stderr: Vec::new(),
    })
}

fn uninstall(root: &Path) -> Result<Answer, String> {
    let Some(installed) = installed(root)? else {
        return Ok(Answer {
            code: EXIT_CLEAN,
            stdout: "weeder guard has installed no hooks in this repository, so there is nothing to take away.\n".to_string(),
            stderr: Vec::new(),
        });
    };

    let mut removed = Vec::new();
    let mut left = Vec::new();
    for entry in &installed {
        let path = root.join(entry);
        match fs::read_if_present(&path).map_err(|error| error.to_string())? {
            // Only a file still carrying weeder's marker is weeder's to remove;
            // whatever someone else put there is theirs.
            Some(script) if guard::binary_named(&script).is_some() => {
                fs::remove_file(&path).map_err(|error| error.to_string())?;
                removed.push(entry.clone());
            }
            Some(_) => left.push(entry.clone()),
            None => {}
        }
    }

    let records = records(root)?;
    let previous = fs::read_if_present(&records.join(PREVIOUS_HOOKS_PATH))
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let previous = previous.trim().to_string();
    let restored = if previous.is_empty() {
        git::config_unset(root, HOOKS_PATH_KEY).map_err(|error| error.to_string())?;
        format!("{HOOKS_PATH_KEY} is unset again")
    } else {
        git::config_set(root, HOOKS_PATH_KEY, &previous).map_err(|error| error.to_string())?;
        format!("{HOOKS_PATH_KEY} is {previous} again")
    };
    fs::remove_file(&records.join(PREVIOUS_HOOKS_PATH)).map_err(|error| error.to_string())?;
    fs::remove_file(&records.join(INSTALLED_HOOKS)).map_err(|error| error.to_string())?;
    fs::remove_dir_if_empty(&records).map_err(|error| error.to_string())?;

    let taken = if removed.is_empty() {
        "no hook weeder wrote was still in place".to_string()
    } else {
        format!("{} removed", removed.join(", "))
    };
    let mut stdout = format!("weeder guard is uninstalled: {taken}, and {restored}.\n");
    for entry in left {
        stdout.push_str(&format!(
            "{entry} is not the hook weeder wrote, so it is still there.\n"
        ));
    }
    Ok(Answer {
        code: EXIT_CLEAN,
        stdout,
        stderr: Vec::new(),
    })
}

/// Pre-commit judges the index before a message exists, so the one allowance a
/// person is entitled to write, a `Weeder-allow:` trailer on this commit, is not
/// on disk yet and this hook can honour none of it. What it does instead is say
/// what blocks and name, for each rule, the exact trailer that would allow it,
/// and leave the verdict to commit-msg, which reads the message the person
/// wrote. The deferral holds only while there is a stage to defer to: with the
/// commit-msg hook gone, pre-commit refuses here, because a gate that hands its
/// verdict to a hook nobody runs is not a gate.
fn pre_commit(root: &Path, version: &str) -> Result<Answer, String> {
    let judged = check::verdict(&judgement(root, version, None, None));
    let answer = judged.answer;
    if answer.code != EXIT_BLOCKED {
        return Ok(answer);
    }
    let named = named_before_the_message(&judged.findings);
    if reads_the_message(root)? {
        return Ok(Answer {
            code: EXIT_CLEAN,
            stdout: format!("{}{named}", answer.stdout),
            ..answer
        });
    }
    Ok(Answer {
        stdout: format!(
            "{}{named}{}",
            answer.stdout,
            refused_line(
                "pre-commit: the index carries a finding that blocks, and the commit-msg hook that reads an allowance is not installed here. repair what the table names, or run weeder guard install and write the allowance on the commit."
            )
        ),
        ..answer
    })
}

/// The commit-msg hook: the index judged again, with the trailers of the message
/// being written honoured. It is the deciding stage for anything a trailer may
/// allow, because it is the first stage at which the person's own words about
/// this change exist.
fn commit_msg(root: &Path, version: &str, request: &CommitMsg) -> Result<Answer, String> {
    let mut judged = judgement(root, version, None, None);
    judged.message_file = Some(request.message_file.clone());
    let judged = check::verdict(&judged);
    let answer = judged.answer;
    if answer.code != EXIT_BLOCKED {
        // pre-commit already wrote the receipt for this very index a moment
        // ago. A second stage saying the same nothing is noise on every commit
        // anyone makes, so this one speaks only when it has something to refuse.
        return Ok(Answer {
            stdout: String::new(),
            ..answer
        });
    }
    Ok(Answer {
        stdout: format!(
            "{}{}{}",
            answer.stdout,
            named_against_the_message(&judged.findings),
            refused_line(
                "commit-msg: the index carries a finding that blocks and this message allows none of it. repair what the table names, or write the allowance above for the change you mean."
            )
        ),
        ..answer
    })
}

/// What pre-commit says about each rule that blocked: the exact trailer a person
/// writes to allow it, and the stage that will read it, since this one cannot.
fn named_before_the_message(findings: &[Finding]) -> String {
    blocking(findings)
        .iter()
        .map(|rule| {
            format!(
                "{rule} is a person's to allow: write `{TRAILER} {rule} <reason>` in this commit's message. pre-commit runs before that message exists, so commit-msg is the stage that reads it.\n"
            )
        })
        .collect()
}

/// What commit-msg says about each rule the message did not allow. The message
/// exists by now, so the line is about what it does not carry rather than about
/// where to write it.
fn named_against_the_message(findings: &[Finding]) -> String {
    blocking(findings)
        .iter()
        .map(|rule| {
            format!(
                "{rule} blocks still: this message carries no `{TRAILER} {rule} <reason>` written for it, and a marker on the line is the agent's own, reported and never honoured.\n"
            )
        })
        .collect()
}

/// The rules that blocked, each named once. They are read off the findings
/// rather than off a list kept beside them, so a rule landing tomorrow teaches
/// its own allowance with nothing added here.
fn blocking(findings: &[Finding]) -> Vec<&str> {
    let mut blocked: Vec<&str> = findings
        .iter()
        .filter(|finding| finding.level == Level::Block)
        .map(|finding| finding.rule.as_str())
        .collect();
    blocked.sort_unstable();
    blocked.dedup();
    blocked
}

/// Whether the commit-msg hook guard installed is still there for git to run.
/// Only weeder's own bundle counts: a file somebody else wrote there answers a
/// different question, and pre-commit would be deferring to it.
fn reads_the_message(root: &Path) -> Result<bool, String> {
    let Some(installed) = installed(root)? else {
        return Ok(false);
    };
    let Some(entry) = installed
        .iter()
        .find(|entry| entry.ends_with(&format!("/{}", Hook::CommitMsg.name())))
    else {
        return Ok(false);
    };
    let path = root.join(entry);
    let Some(script) = fs::read_if_present(&path).map_err(|error| error.to_string())? else {
        return Ok(false);
    };
    Ok(guard::is_own_bundle(Hook::CommitMsg, &script) && fs::is_executable(&path))
}

fn pre_push(root: &Path, version: &str, request: &PrePush) -> Result<Answer, String> {
    let protected = protected(root, &request.protect)?;
    let remote = request
        .arguments
        .first()
        .map(String::as_str)
        .unwrap_or("the remote");

    let mut stdout = String::new();
    let mut refused = false;
    for pushed in guard::parse_push_refs(&request.refs) {
        let branch = guard::short_branch(&pushed.remote_ref);
        let is_protected = guard::protects(&protected, &pushed.remote_ref);

        if pushed.deletes() {
            if is_protected {
                refused = true;
                stdout.push_str(&refused_line(&format!(
                    "{branch} is protected, and this push would take it off {remote} altogether. take {branch} out of [guard] protected in weeder.toml if that is really the intent."
                )));
            }
            continue;
        }

        if is_protected
            && !pushed.creates()
            && !git::is_ancestor(root, &pushed.remote_sha, &pushed.local_sha)
                .map_err(|error| error.to_string())?
        {
            refused = true;
            stdout.push_str(&refused_line(&format!(
                "{branch} is protected, and this push is not a fast-forward, so it would drop commits {remote} already has. rebase on {branch} as the remote holds it, or push a branch of your own."
            )));
            continue;
        }

        let base = base_for(root, &pushed)?;
        let answer = check::run(&judgement(
            root,
            version,
            Some(base.clone()),
            Some(pushed.local_sha.clone()),
        ));
        if answer.code == EXIT_COULD_NOT_RUN {
            return Err(answer.stderr.join(" "));
        }
        stdout.push_str(&answer.stdout);
        if answer.code == EXIT_BLOCKED {
            refused = true;
            stdout.push_str(&refused_line(&format!(
                "what {branch} carries beyond {} has a finding that blocks. repair it, commit the repair, and push again.",
                abbreviate(&base)
            )));
        }
    }

    Ok(Answer {
        code: if refused { EXIT_BLOCKED } else { EXIT_CLEAN },
        stdout,
        stderr: Vec::new(),
    })
}

fn pre_rebase(root: &Path, request: &PreRebase) -> Result<Answer, String> {
    let protected = protected(root, &request.protect)?;
    // A rebase rewrites the branch it is given, or the one that is checked out
    // where it is given none. The upstream is only read from, so a protected
    // upstream is the ordinary case, rebasing your own branch onto main, and
    // is not history this hook is here to keep.
    let branch = match request.arguments.get(1) {
        Some(branch) => Some(branch.clone()),
        None => git::current_branch(root).map_err(|error| error.to_string())?,
    };
    let Some(branch) = branch else {
        return Ok(allowed());
    };
    if !guard::protects(&protected, &branch) {
        return Ok(allowed());
    }

    let branch = guard::short_branch(&branch);
    let onto = match request.arguments.first() {
        Some(upstream) => format!(", and rebasing it onto {upstream} rewrites its history"),
        None => ", and a rebase rewrites its history".to_string(),
    };
    Ok(Answer {
        code: EXIT_BLOCKED,
        stdout: refused_line(&format!(
            "{branch} is protected{onto}. rebase a branch of your own, or take {branch} out of [guard] protected in weeder.toml."
        )),
        stderr: Vec::new(),
    })
}

/// What a hook asks `weeder check`: the index alone at pre-commit and commit-msg
/// time, a range at pre-push time, and always `--strict`, so an allowance an
/// agent wrote itself on a line is reported and not honoured. A trailer is the
/// other thing: it is a person's act, on the very change being judged, and every
/// hook honours one it can see. pre-commit sees none, because no message exists
/// yet. The table is what git prints, because whoever is being refused is a person.
fn judgement(
    root: &Path,
    version: &str,
    base: Option<String>,
    tip: Option<String>,
) -> check::Request {
    check::Request {
        cwd: root.to_path_buf(),
        staged: base.is_none(),
        base,
        tip,
        scope: Vec::new(),
        // A commit carries the index and a push carries commits, so neither
        // hook has a working tree to take an unstaged file from.
        untracked: None,
        strict: true,
        honour_trailers: true,
        format: crate::faces::Format::Table,
        config: None,
        message_file: None,
        version: version.to_string(),
    }
}

/// What the pushed commits are judged against: the commit the remote ref points
/// at, or, where the two have diverged, the last commit they share. A ref no
/// remote has yet is judged from where its unpublished commits begin, and a
/// history that has never been pushed anywhere is judged whole.
fn base_for(root: &Path, pushed: &PushRef) -> Result<String, String> {
    if !pushed.creates() {
        let shared = git::merge_base(root, &pushed.remote_sha, &pushed.local_sha)
            .map_err(|error| error.to_string())?;
        if let Some(shared) = shared {
            return Ok(shared);
        }
    }
    Ok(git::unpublished_boundary(root, &pushed.local_sha)
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| git::EMPTY_TREE.to_string()))
}

/// The branches these hooks protect: what `install --protect` baked into the
/// bundle, and otherwise what `weeder.toml` says at the moment git runs the hook,
/// so changing the config takes effect without installing again.
fn protected(root: &Path, from_the_bundle: &[String]) -> Result<Vec<String>, String> {
    if !from_the_bundle.is_empty() {
        return Ok(from_the_bundle.to_vec());
    }
    Ok(read_config(root, None)?.protected_branches)
}

/// The line `install` prints about what the hooks will keep, which is the one
/// thing a person cannot read off the file list.
fn protection(root: &Path, from_the_flag: &[String]) -> Result<String, String> {
    let branches = protected(root, from_the_flag)?;
    let source = if from_the_flag.is_empty() {
        "as weeder.toml has them today"
    } else {
        "as this install named them"
    };
    Ok(format!(
        "protected: {} ({source}).\n",
        if branches.is_empty() {
            "no branch".to_string()
        } else {
            branches.join(", ")
        }
    ))
}

/// The hooks this repository's record says guard wrote, or `None` where guard
/// has written none.
fn installed(root: &Path) -> Result<Option<Vec<String>>, String> {
    let record = records(root)?.join(INSTALLED_HOOKS);
    let Some(text) = fs::read_if_present(&record).map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let entries: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    if entries.is_empty() {
        return Ok(None);
    }
    Ok(Some(entries))
}

fn records(root: &Path) -> Result<PathBuf, String> {
    let directory = git::git_dir(root).map_err(|error| error.to_string())?;
    Ok(directory.join(RECORD_DIR))
}

/// The directory the recorded hooks sit in, as the record spells it.
fn directory_of(installed: &[String]) -> &str {
    installed
        .first()
        .and_then(|entry| entry.rsplit_once('/'))
        .map(|(directory, _)| directory)
        .unwrap_or(DEFAULT_HOOKS_DIR)
}

/// Whether a `core.hooksPath` setting names the directory guard installed into.
/// Both are resolved against the repository first, so `.githooks` and an
/// absolute path to the same place read as the same place.
fn same_directory(root: &Path, setting: &str, directory: &str) -> bool {
    match (
        fs::canonical(&root.join(setting)),
        fs::canonical(&root.join(directory)),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn refused_line(reason: &str) -> String {
    format!("weeder guard refused: {reason}\n")
}

fn allowed() -> Answer {
    Answer {
        code: EXIT_CLEAN,
        stdout: String::new(),
        stderr: Vec::new(),
    }
}

fn missing(reason: &str) -> Answer {
    Answer {
        code: EXIT_BLOCKED,
        stdout: format!("{reason}\n"),
        stderr: Vec::new(),
    }
}

/// A commit named the way a person reads one back.
fn abbreviate(commit: &str) -> &str {
    match commit.char_indices().nth(12) {
        Some((index, _)) => &commit[..index],
        None => commit,
    }
}

fn counted(misses: usize) -> String {
    if misses == 1 {
        "1 miss".to_string()
    } else {
        format!("{misses} misses")
    }
}
