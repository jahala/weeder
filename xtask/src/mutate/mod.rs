//! The recall campaign: what weed catches when the anti-pattern is really there.
//!
//! Calibration asks how often weed blocks a good commit. This asks the mirror
//! question, and it has to be asked on the same code, or the answer is about
//! fixtures rather than about work. So the campaign walks the commits
//! calibration walks, plants exactly one anti-pattern in each case, runs the
//! binary the way a gate runs it, and writes down whether the rule fired on that
//! site and nowhere else.
//!
//! Three things keep the number honest. The injector finds its sites with its
//! own scanner, never with weed's reader. A site the unmutated commit already
//! fires that rule on is not used, so a hit is never something the commit
//! brought with it. And every miss is written down by repository, commit and
//! site, with its before and after kept on disk, so a number nobody believes can
//! be replayed by hand.

mod corpus;
mod git;
mod inject;
mod judge;
mod report;
mod source;
mod tree;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clap::Args;

use corpus::Working;
use inject::{Mutation, NoSite};
use source::Language;
use tree::Tree;

#[derive(Args, Debug, Clone)]
pub struct Request {
    /// How many cases each rule is given in each language before the campaign
    /// moves on.
    #[arg(long, default_value_t = 40)]
    pub cases: usize,
    /// How far back each repository is walked.
    #[arg(long, default_value_t = 200)]
    pub commits: usize,
    /// The file the recall section is written into.
    #[arg(long, default_value = "docs/calibration-2026-09.md")]
    pub out: PathBuf,
    /// Only these rules.
    #[arg(long = "rule")]
    pub rules: Vec<String>,
    /// Only these repositories.
    #[arg(long = "repo")]
    pub repos: Vec<String>,
    /// Only these languages.
    #[arg(long = "language")]
    pub languages: Vec<String>,
    /// Where the before and after of every miss is kept.
    #[arg(long)]
    pub cases_dir: Option<PathBuf>,
    /// Write the whole run as json as well, for anything that would rather read
    /// numbers than prose.
    #[arg(long)]
    pub json: Option<PathBuf>,
    /// The weed to run. The release binary of this workspace by default.
    #[arg(long)]
    pub binary: Option<PathBuf>,
}

/// One planted anti-pattern and what weed did about it.
#[derive(Debug, Clone)]
pub struct Case {
    pub rule: String,
    pub lang: Language,
    pub repo: String,
    pub sha: String,
    pub path: String,
    pub line: Option<u32>,
    pub shape: String,
    pub caught: bool,
}

/// What one repository gave the campaign.
#[derive(Debug, Default)]
pub struct Outcome {
    pub repo: String,
    pub reference: String,
    pub head: String,
    pub walked: usize,
    pub cases: Vec<Case>,
    /// Commits that held no site for a rule in a language.
    pub absent: BTreeMap<(String, Language), usize>,
    /// Rules a language cannot be made to break at all.
    pub impossible: BTreeSet<(String, Language)>,
    /// Sites passed over because the commit itself already fires that rule
    /// there, where a hit would prove nothing.
    pub passed_over: BTreeMap<(String, Language), usize>,
    /// Files that are no site at all, by the reason, counted over every commit
    /// the walk read.
    pub no_site: BTreeMap<tree::Passed, usize>,
}

pub fn run(request: &Request) -> Result<(), String> {
    let root = workspace_root();
    let binary = weed_binary(request, &root)?;
    let levels = report::levels(&binary)?;
    let cases_dir = request
        .cases_dir
        .clone()
        .unwrap_or_else(|| corpus::cache_root().join("cases"));

    let sources: Vec<&corpus::Source> = corpus::CORPUS
        .iter()
        .filter(|source| request.repos.is_empty() || request.repos.iter().any(|n| n == source.name))
        .collect();
    if sources.is_empty() {
        return Err("no repository of the corpus was named".to_string());
    }

    let plan = Plan::new(request, &sources);
    let mut outcomes = Vec::new();
    let mut failures = Vec::new();
    std::thread::scope(|scope| {
        let mut running = Vec::new();
        for source in &sources {
            let plan = &plan;
            let binary = binary.clone();
            let cases_dir = cases_dir.clone();
            running.push((
                source.name,
                scope.spawn(move || walk(source, plan, &binary, &cases_dir)),
            ));
        }
        for (name, handle) in running {
            match handle.join() {
                Ok(Ok(outcome)) => outcomes.push(outcome),
                Ok(Err(reason)) => failures.push(format!("{name}: {reason}")),
                Err(_) => failures.push(format!("{name}: the walk panicked")),
            }
        }
    });
    if !failures.is_empty() {
        return Err(failures.join("\n"));
    }
    outcomes.sort_by(|one, other| one.repo.cmp(&other.repo));

    let written = report::write(&request.out, &outcomes, &levels, &plan)?;
    if let Some(path) = &request.json {
        report::write_json(path, &outcomes)?;
    }
    println!("{written}");
    Ok(())
}

/// What the campaign is asking for: the rules, the languages, and how many
/// cases each repository owes.
pub struct Plan {
    pub rules: Vec<String>,
    pub languages: Vec<Language>,
    pub commits: usize,
    /// How many cases one repository owes for one rule in one language.
    quota: BTreeMap<(&'static str, Language), usize>,
}

impl Plan {
    fn new(request: &Request, sources: &[&corpus::Source]) -> Plan {
        let rules: Vec<String> = inject::RULES
            .iter()
            .filter(|rule| {
                request.rules.is_empty()
                    || request
                        .rules
                        .iter()
                        .any(|named| named.eq_ignore_ascii_case(rule))
            })
            .map(ToString::to_string)
            .collect();
        let languages: Vec<Language> = Language::ALL
            .into_iter()
            .filter(|lang| {
                request.languages.is_empty()
                    || request.languages.iter().any(|named| named == lang.slug())
            })
            .collect();
        // The cases for a language are shared out between the repositories that
        // write it, so three TypeScript repositories each carry a third and the
        // measurement is never one repository's habits.
        let mut quota = BTreeMap::new();
        for lang in &languages {
            let serving: Vec<&corpus::Source> = sources
                .iter()
                .filter(|source| source.languages.contains(lang))
                .copied()
                .collect();
            let each = request.cases.div_ceil(serving.len().max(1));
            for source in serving {
                quota.insert((source.name, *lang), each);
            }
        }
        Plan {
            rules,
            languages,
            commits: request.commits,
            quota,
        }
    }

    fn owed(&self, repo: &str, lang: Language) -> usize {
        self.quota
            .iter()
            .find(|((name, held), _)| *name == repo && *held == lang)
            .map_or(0, |(_, owed)| *owed)
    }
}

/// One repository, walked newest commit first until every rule it serves has
/// the cases it owes.
fn walk(
    source: &corpus::Source,
    plan: &Plan,
    binary: &Path,
    cases_dir: &Path,
) -> Result<Outcome, String> {
    let working = corpus::prepare(source)?;
    let languages: Vec<Language> = plan
        .languages
        .iter()
        .filter(|lang| source.languages.contains(lang))
        .copied()
        .collect();
    let mut outcome = Outcome {
        repo: source.name.to_string(),
        reference: working.reference.clone(),
        head: working.head.clone(),
        ..Outcome::default()
    };
    if languages.is_empty() {
        return Ok(outcome);
    }

    let mut wanted: BTreeMap<(String, Language), usize> = BTreeMap::new();
    for rule in &plan.rules {
        for lang in &languages {
            wanted.insert((rule.clone(), *lang), plan.owed(source.name, *lang));
        }
    }

    let history = git::lines(
        &working.root,
        &[
            "log",
            "--first-parent",
            "--format=%H %P",
            "-n",
            &plan.commits.to_string(),
            &working.reference,
        ],
    )?;
    let config = corpus::cache_root().join(format!("{}.weed.toml", source.name));

    for entry in history {
        if wanted.values().all(|owed| *owed == 0) {
            break;
        }
        let mut parts = entry.split_whitespace();
        let (Some(sha), Some(parent)) = (parts.next(), parts.next()) else {
            // A root commit has no parent to judge against.
            continue;
        };
        let (sha, parent) = (sha.to_string(), parent.to_string());
        git::run(
            &working.root,
            &["checkout", "--quiet", "--force", "--detach", &sha],
        )?;
        outcome.walked += 1;
        let paths = git::lines(&working.root, &["ls-files"])?;
        let changed = git::lines(
            &working.root,
            &["diff", "--name-only", "--find-renames", &parent, &sha],
        )?;
        let added = git::lines(
            &working.root,
            &["diff", "--name-only", "--diff-filter=A", &parent, &sha],
        )?;
        let tree = Tree::read(&working.root, paths, changed, &added, &languages);
        for (reason, counted) in &tree.passed_over {
            *outcome.no_site.entry(*reason).or_default() += counted;
        }
        let baseline = judge::check(binary, &working.root, &parent, &[])?;

        for rule in &plan.rules {
            for lang in &languages {
                let key = (rule.clone(), *lang);
                if wanted.get(&key).copied().unwrap_or_default() == 0 {
                    continue;
                }
                let seed = seed_of(&sha, rule, *lang);
                let planted = match inject::inject(rule, *lang, &tree, seed) {
                    Ok(planted) => planted,
                    Err(NoSite::Absent) => {
                        *outcome.absent.entry(key).or_default() += 1;
                        continue;
                    }
                    Err(NoSite::NotInLanguage) => {
                        outcome.impossible.insert(key);
                        *wanted.entry((rule.clone(), *lang)).or_default() = 0;
                        continue;
                    }
                };
                // A site the commit itself already lights up proves nothing:
                // the finding would be there without the injection.
                let against = if planted.arguments.is_empty() && planted.config.is_none() {
                    baseline.clone()
                } else {
                    prepare_config(&planted, &config)?;
                    judge::check(
                        binary,
                        &working.root,
                        &parent,
                        &arguments(&planted, &config),
                    )?
                };
                if against
                    .iter()
                    .any(|found| found.rule == *rule && planted.target.contains(&found.path))
                {
                    *outcome.passed_over.entry(key).or_default() += 1;
                    continue;
                }

                let before = apply(&working, &planted)?;
                let found = judge::check(
                    binary,
                    &working.root,
                    &parent,
                    &arguments(&planted, &config),
                );
                let restored = restore(&working, &planted, &tree);
                let found = found?;
                restored?;

                let caught = caught(&found, &planted, rule);
                if !caught {
                    report::keep(cases_dir, source.name, &sha, rule, *lang, &planted, &before)?;
                }
                outcome.cases.push(Case {
                    rule: rule.clone(),
                    lang: *lang,
                    repo: source.name.to_string(),
                    sha: sha.clone(),
                    path: planted.target.first().cloned().unwrap_or_default(),
                    line: planted.lines.map(|(first, _)| first),
                    shape: planted.shape.clone(),
                    caught,
                });
                if let Some(owed) = wanted.get_mut(&key) {
                    *owed = owed.saturating_sub(1);
                }
            }
        }
    }
    Ok(outcome)
}

/// Whether weed reported this rule on the site the injector planted it in.
fn caught(found: &[judge::Finding], planted: &Mutation, rule: &str) -> bool {
    found.iter().any(|finding| {
        finding.rule == rule
            && planted.target.contains(&finding.path)
            && match (planted.lines, finding.line) {
                // A shape written on lines is only caught where the finding
                // points at them: a rule that fires on the same file for
                // another reason has not caught this.
                (Some((first, last)), Some(line)) => line + 1 >= first && line <= last + 1,
                (Some(_), None) => false,
                (None, _) => true,
            }
    })
}

fn arguments(planted: &Mutation, config: &Path) -> Vec<String> {
    let mut arguments = planted.arguments.clone();
    if planted.config.is_some() {
        arguments.push("--config".to_string());
        arguments.push(config.to_string_lossy().to_string());
    }
    arguments
}

/// The config a case reads, written outside the repository so that asking for
/// layers is not itself a change to the tree being judged.
fn prepare_config(planted: &Mutation, config: &Path) -> Result<(), String> {
    let Some(text) = &planted.config else {
        return Ok(());
    };
    std::fs::write(config, text).map_err(|error| format!("{}: {error}", config.display()))
}

/// Write the mutation into the working tree and stage it, so the diff git shows
/// carries a file the tree never had. The text each touched file held before is
/// answered, for the pair a miss is kept as.
fn apply(working: &Working, planted: &Mutation) -> Result<Vec<(String, Option<String>)>, String> {
    let mut before = Vec::new();
    for (path, content) in &planted.writes {
        let whole = working.root.join(path);
        before.push((path.clone(), std::fs::read_to_string(&whole).ok()));
        match content {
            Some(text) => {
                if let Some(parent) = whole.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| format!("{}: {error}", parent.display()))?;
                }
                std::fs::write(&whole, text)
                    .map_err(|error| format!("{}: {error}", whole.display()))?;
            }
            None => {
                git::run(
                    &working.root,
                    &["rm", "--quiet", "--force", "--ignore-unmatch", "--", path],
                )?;
            }
        }
    }
    git::run(&working.root, &["add", "--all"])?;
    Ok(before)
}

/// Put the tree back exactly as the commit left it. A file the injection added
/// is one git has been told about, so git is what takes it back out; everything
/// else is restored from the commit.
fn restore(working: &Working, planted: &Mutation, tree: &Tree) -> Result<(), String> {
    for (path, _) in &planted.writes {
        if !tree.paths.iter().any(|held| held == path) {
            git::run(
                &working.root,
                &["rm", "--quiet", "--force", "--ignore-unmatch", "--", path],
            )?;
        }
    }
    git::run(
        &working.root,
        &["checkout", "--quiet", "--force", "HEAD", "--", "."],
    )
}

/// The number that decides which site a commit offers, so two runs of the
/// campaign plant the same anti-pattern in the same place.
fn seed_of(sha: &str, rule: &str, lang: Language) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in sha.bytes().chain(rule.bytes()).chain(lang.slug().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/// The workspace this xtask was built in.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// The binary the campaign judges with: the one asked for, or the release build
/// of this workspace, built if it is not there.
fn weed_binary(request: &Request, root: &Path) -> Result<PathBuf, String> {
    if let Some(named) = &request.binary {
        return named
            .exists()
            .then(|| named.clone())
            .ok_or_else(|| format!("{} is not a file", named.display()));
    }
    // Always built, never merely found: a campaign that judged with yesterday's
    // binary would report yesterday's recall.
    let built = root.join("target/release/weed");
    let status =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
            .current_dir(root)
            .args(["build", "--release", "--bin", "weed"])
            .status()
            .map_err(|error| format!("cargo build: {error}"))?;
    if !status.success() {
        return Err("the release binary could not be built".to_string());
    }
    built
        .exists()
        .then_some(built)
        .ok_or_else(|| "target/release/weed is not there to run".to_string())
}
