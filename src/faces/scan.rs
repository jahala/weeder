//! `weed scan`, the tree judged.
//!
//! `check` asks what a change did. `scan` asks what the repository has become:
//! documentation that cites what is gone, exports nothing calls, work markers
//! nobody came back to, pins the registries left behind. None of that is any one
//! change's fault, so none of it may stop one. `scan` reports at warning level
//! and leaves with 0, or with 3 when it could not run at all.
//!
//! What it reads is the tree as it sits: tracked files, and untracked files the
//! ignore rules do not hide. Whether a change is staged, unstaged or committed
//! makes no difference to a question about the state a repository is in.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::core::catalogue::{self, Face};
use crate::core::classify::{classify_file, Lang};
use crate::core::config::Config;
use crate::core::finding::Finding;
use crate::core::help;
use crate::core::read::Outline;
use crate::core::registry::{self, parse_snapshot, Registry, Snapshot, SNAPSHOT_PATH};
use crate::core::rules;
use crate::core::sarif::{self, Context, EXIT_CLEAN, EXIT_COULD_NOT_RUN, RULES_DOC};
use crate::core::tree::{CommandListing, Tree, TreeFile};
use crate::faces::{read_config, Answer, Format};
use crate::seams::{exec, fs, git, reader};

/// How long weed waits for a command whose help it was told to read. A help
/// listing is printed and gone; anything slower is a command that is doing
/// something else.
const HELP_TIMEOUT: Duration = Duration::from_secs(5);

/// How long weed waits for a registry to answer during a refresh.
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);

/// How deep the walk of a command's own help goes. Three levels reach
/// `weed guard install`, and a fourth is there so a deeper CLI is read whole
/// rather than reported as though its subcommands had gone.
const HELP_DEPTH: usize = 4;

/// The program a refresh fetches through. weed shells out to git already; a
/// fetch is the same kind of question asked of a different tool, and keeping it
/// out of the binary is what keeps the binary a judge rather than a client.
const FETCH_PROGRAM: &str = "curl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Where weed was called from; the repository is found from here.
    pub cwd: PathBuf,
    /// The rule ids to run. Empty runs every scan rule the config leaves on.
    pub rules: Vec<String>,
    pub format: Format,
    /// Read `weed.toml` from here instead of the repository root.
    pub config: Option<PathBuf>,
    /// Ask the registries for the latest release of everything the manifests
    /// pin, write `.weed/registry-snapshot.json`, and scan against it.
    pub refresh_snapshot: bool,
    pub version: String,
}

pub fn run(request: &Request) -> Answer {
    match judge(request) {
        Ok(answer) => answer,
        Err(reason) => could_not_run(request, &reason),
    }
}

fn judge(request: &Request) -> Result<Answer, String> {
    let root = git::repository_root(&request.cwd).map_err(|error| error.to_string())?;
    let config = read_config(&root, request.config.as_deref())?;
    let wanted = wanted_rules(&request.rules)?;

    let mut unreachable = Vec::new();
    if request.refresh_snapshot {
        unreachable = refresh(&root, &request.version)?;
    }

    let mut tree = gather(&root, &config)?;
    // R3 dates a line through git, and only the files carrying a work marker
    // are worth asking about. The rule names them; the face is what leaves the
    // process to find out.
    if runs("R3", &config, &wanted) {
        for path in rules::scan::r3::marked_files(&tree) {
            let times = git::blame_line_times(&root, &path).map_err(|error| error.to_string())?;
            tree.blame.insert(path, times);
        }
    }

    let findings = rules::scan::evaluate(&tree, &config, &wanted);
    Ok(Answer {
        // A scan never blocks. It reports the state a repository is in, and no
        // reading of that state is a reason to stop a change from landing.
        code: EXIT_CLEAN,
        stdout: write(&findings, request, &root),
        stderr: unreachable,
    })
}

/// The rule ids this run is narrowed to. An id weed does not know, and a check
/// rule asked of the scan face, are both a run that never happened: weed will
/// not quietly report on a smaller question than it was asked.
fn wanted_rules(asked: &[String]) -> Result<Vec<String>, String> {
    let mut wanted = Vec::new();
    for id in asked.iter().flat_map(|listed| listed.split(',')) {
        let id = id.trim().to_ascii_uppercase();
        if id.is_empty() {
            continue;
        }
        match catalogue::rule(&id) {
            Some(rule) if rule.face == Face::Scan => wanted.push(id),
            Some(_) => {
                return Err(format!(
                    "{id} is a check rule, and weed scan runs the scan rules. run it with weed check, or pass --rules a scan rule."
                ))
            }
            None => {
                return Err(format!(
                    "weed knows no rule called {id}. run weed rules to see the catalogue."
                ))
            }
        }
    }
    Ok(wanted)
}

/// Whether a rule runs at all this time: the config leaves it on, and the
/// caller either named it or named nothing.
fn runs(id: &str, config: &Config, wanted: &[String]) -> bool {
    if !wanted.is_empty() && !wanted.iter().any(|asked| asked == id) {
        return false;
    }
    catalogue::rule(id)
        .and_then(|rule| rules::configured_level(rule, config))
        .is_some()
}

/// The repository as it sits, with everything a rule needs already read. This is
/// the one place a scan touches a disk, a clock or another process.
fn gather(root: &Path, config: &Config) -> Result<Tree, String> {
    let paths = git::tree_files(root).map_err(|error| error.to_string())?;
    let files = paths
        .into_iter()
        .map(|path| read_file(root, path))
        .collect::<Result<Vec<TreeFile>, String>>()?;

    let snapshot = read_snapshot(root)?;
    let commands = listings(root, &config.doc_commands)?;

    Ok(Tree {
        files,
        blame: std::collections::BTreeMap::new(),
        now: now(),
        commands,
        snapshot,
    })
}

fn read_file(root: &Path, path: String) -> Result<TreeFile, String> {
    let content = git::file_in_tree(root, &path).map_err(|error| error.to_string())?;
    let classification = content
        .as_deref()
        .map(|content| classify_file(&path, content));
    // Only a file in a language weed reads has anything to outline, and the
    // outline is the expensive half of reading a tree.
    let outline = match (&content, &classification) {
        (Some(content), Some(classification)) if classification.lang != Lang::Other => {
            reader::outline(Path::new(&path), content)
        }
        _ => Outline::default(),
    };
    Ok(TreeFile {
        path,
        content,
        classification,
        outline,
    })
}

/// The committed snapshot, read from disk rather than from the listing: a
/// repository is free to keep `.weed/` out of git, and the file is still the
/// answer R4 measures against.
fn read_snapshot(root: &Path) -> Result<Snapshot, String> {
    let text = fs::read_if_present(&root.join(SNAPSHOT_PATH)).map_err(|error| error.to_string())?;
    parse_snapshot(text.as_deref()).map_err(|error| error.to_string())
}

/// The moment the scan started, seconds since the epoch. A clock before the
/// epoch is a machine weed has nothing sensible to say about, and reads as the
/// epoch itself rather than as a failure.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

/// The help of every command `[docs] commands` names, walked down through its
/// subcommands. A command the config names and this machine cannot run is a
/// scan that never happened: R1 would otherwise report every citation of it as
/// gone.
fn listings(root: &Path, commands: &[String]) -> Result<Vec<CommandListing>, String> {
    let mut found = Vec::new();
    for command in commands {
        walk(root, std::slice::from_ref(command), &mut found)?;
    }
    Ok(found)
}

fn walk(root: &Path, path: &[String], into: &mut Vec<CommandListing>) -> Result<(), String> {
    if path.len() > HELP_DEPTH || into.iter().any(|listing| listing.path == path) {
        return Ok(());
    }
    let mut arguments: Vec<&str> = path[1..].iter().map(String::as_str).collect();
    arguments.push("--help");
    let printed = exec::run(root, &path[0], &arguments, HELP_TIMEOUT)
        .map_err(|error| format!("{error} [docs] commands named {}.", path[0]))?;
    // A generator writes its help to whichever stream it likes, and a command
    // that refuses a subcommand still prints what it does accept.
    let listing = help::parse(&format!("{}{}", printed.stdout, printed.stderr));
    into.push(CommandListing {
        path: path.to_vec(),
        subcommands: listing.subcommands.clone(),
        flags: listing.flags,
    });
    for subcommand in listing.subcommands {
        let mut deeper = path.to_vec();
        deeper.push(subcommand);
        walk(root, &deeper, into)?;
    }
    Ok(())
}

/// Ask each registry what its latest release is, and write the snapshot the
/// repository commits. One line comes back for every package weed could not
/// get an answer about; a scan then runs against what it did get.
fn refresh(root: &Path, version: &str) -> Result<Vec<String>, String> {
    let mut wanted: Vec<(Registry, String)> = Vec::new();
    for path in git::tree_files(root).map_err(|error| error.to_string())? {
        let Some(registry) = registry::registry_of(&path) else {
            continue;
        };
        let Some(content) = git::file_in_tree(root, &path).map_err(|error| error.to_string())?
        else {
            continue;
        };
        for pin in registry::pins(registry, &content) {
            let entry = (registry, pin.package);
            if !wanted.contains(&entry) {
                wanted.push(entry);
            }
        }
    }

    let agent = format!("weed/{version}");
    let mut snapshot = Snapshot::default();
    let mut unreachable = Vec::new();
    for (registry, package) in &wanted {
        match fetch(root, *registry, package, &agent) {
            Ok(Some(latest)) => snapshot.record(*registry, package, &latest),
            Ok(None) => unreachable.push(format!(
                "{} had no latest release to give for {package}, so the snapshot keeps nothing for it.",
                registry.name()
            )),
            Err(error) => return Err(error),
        }
    }

    let file = root.join(SNAPSHOT_PATH);
    if let Some(directory) = file.parent() {
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    fs::write(&file, &snapshot.to_json()).map_err(|error| error.to_string())?;
    Ok(unreachable)
}

/// One registry, asked about one package. A registry that refuses or answers
/// something weed cannot read is a package with no latest release; a fetcher
/// this machine does not have is a refresh that never happened.
fn fetch(
    root: &Path,
    registry: Registry,
    package: &str,
    agent: &str,
) -> Result<Option<String>, String> {
    let url = registry.latest_url(package);
    let seconds = FETCH_TIMEOUT.as_secs().to_string();
    let arguments = [
        "--silent",
        "--show-error",
        "--location",
        "--max-time",
        &seconds,
        "--header",
        "Accept: application/json",
        "--user-agent",
        agent,
        &url,
    ];
    let answer = exec::run(root, FETCH_PROGRAM, &arguments, FETCH_TIMEOUT)
        .map_err(|error| format!("{error} --refresh-snapshot is the one thing weed does over the network, and it asks {FETCH_PROGRAM} to do it."))?;
    if answer.code != 0 {
        return Ok(None);
    }
    Ok(registry.latest_in(&answer.stdout))
}

fn write(findings: &[Finding], request: &Request, root: &Path) -> String {
    match request.format {
        Format::Table => sarif::render_table(findings),
        Format::Sarif => {
            let mut context = Context::new(&request.version).with_catalogue(scan_catalogue());
            if fs::exists(&root.join(RULES_DOC)) {
                context = context.with_docs_base(format!("file://{}", root.display()));
            }
            format!("{}\n", sarif::to_json(&sarif::render(findings, &context)))
        }
    }
}

/// The rules a scan log declares: the ones this face can report. A consumer
/// reading the tool component learns what weed was looking for, and a check
/// rule was never among them.
fn scan_catalogue() -> Vec<catalogue::Rule> {
    catalogue::rules()
        .iter()
        .filter(|rule| rule.face == Face::Scan)
        .copied()
        .collect()
}

fn could_not_run(request: &Request, reason: &str) -> Answer {
    let context = Context::new(&request.version)
        .with_catalogue(scan_catalogue())
        .could_not_run(reason);
    Answer {
        code: EXIT_COULD_NOT_RUN,
        stdout: match request.format {
            Format::Sarif => format!("{}\n", sarif::to_json(&sarif::render(&[], &context))),
            Format::Table => String::new(),
        },
        stderr: vec![reason.to_string()],
    }
}
