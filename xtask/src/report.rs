//! `docs/calibration-2026-09.md`, written from the measurement and the ledger.
//!
//! The verdict is the first sentence, because a reader who stops after one line
//! must still have the answer, and the sentence after it says whether the
//! re-grade that number rests on was taken sighted or blind. Then the method,
//! then the levels every rule ran at, then one table per repository, then the
//! totals, then precision beside the allowance rate, then the rule-level view. The file carries no date and no
//! duration: two runs over the same history write the same bytes, so a diff of
//! this file is a change in weeder's judgement and never in the weather.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use weeder::core::catalogue::{self, Face};
use weeder::core::config::Config;
use weeder::core::finding::Level;
use weeder::core::rules::configured_level;

use crate::audit::Audit;
use crate::calibrate::{RepoMeasurement, REPORTED_ONLY, WINDOW};
use crate::judgement::{Classification, Ledger};
use crate::split::{self, Split};
use crate::suppressions::RepoRate;

/// The share of judged commits that block-level false positives may take before
/// weeder is not a gate. It is read over the pooled total, not per repository: one
/// small repository having a bad week is not the question a gate answers.
pub const BAR: f64 = 2.0;

/// The rules the kill bar names. Reaching the bar with any of them turned down
/// is not reaching the bar, so their levels are written into the file and read
/// back out of it.
pub const LOAD_BEARING: [&str; 4] = ["T1", "T2", "T3", "S1"];

/// How one repository's blocks were classified.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub true_positive: usize,
    pub acceptable: usize,
    pub false_positive: usize,
    /// Blocked commits nobody has classified. They count as false positives
    /// everywhere a number is drawn, so the bar cannot be reached by leaving a
    /// commit out of the ledger.
    pub unclassified: usize,
}

impl Tally {
    pub fn blocked(&self) -> usize {
        self.true_positive + self.acceptable + self.false_positive + self.unclassified
    }

    /// What counts against the bar.
    pub fn against_the_bar(&self) -> usize {
        self.false_positive + self.unclassified
    }

    fn add(&mut self, classification: Option<Classification>) {
        match classification {
            Some(Classification::TruePositive) => self.true_positive += 1,
            Some(Classification::Acceptable) => self.acceptable += 1,
            Some(Classification::FalsePositive) => self.false_positive += 1,
            None => self.unclassified += 1,
        }
    }
}

/// Everything the file is written from.
pub struct Report<'a> {
    /// The day the audited question changed, printed beside the floor so both
    /// numbers stay in the file; none where the question never changed.
    pub ruling: Option<&'a crate::ruling::Ruling>,
    pub repos: &'a [RepoMeasurement],
    pub rates: &'a [RepoRate],
    pub ledger: &'a Ledger,
    /// What the rules moved since the run whose blocks were written down.
    pub split: &'a Split,
    /// The independent re-grade, which decides how the verdict is worded.
    pub audit: &'a Audit,
}

/// The numbers a caller wants back without reading the markdown: what the run
/// came to, for the command line to print and for a test to assert on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub judged: usize,
    pub blocked: usize,
    pub tally: Tally,
    pub refusals: usize,
    pub ships: bool,
    /// How many blocked commits count against the bar once every
    /// `false-positive` a blind re-grade recorded is taken as true. It is never
    /// below `tally.against_the_bar()`, and the two are equal where no blind
    /// re-grade has read a block back.
    pub floor: usize,
}

impl<'a> Report<'a> {
    /// Every judged commit across the corpus.
    fn judged(&self) -> usize {
        self.repos.iter().map(|repo| repo.judged).sum()
    }

    fn pooled(&self) -> Tally {
        let mut pooled = Tally::default();
        for repo in self.repos {
            let tally = self.tally(repo);
            pooled.true_positive += tally.true_positive;
            pooled.acceptable += tally.acceptable;
            pooled.false_positive += tally.false_positive;
            pooled.unclassified += tally.unclassified;
        }
        pooled
    }

    fn tally(&self, repo: &RepoMeasurement) -> Tally {
        let mut tally = Tally::default();
        for blocked in &repo.blocked {
            tally.add(
                self.ledger
                    .get(&repo.name, &blocked.sha)
                    .map(|judgement| judgement.classification),
            );
        }
        tally
    }

    fn refusals(&self) -> usize {
        self.repos.iter().map(|repo| repo.refusals.len()).sum()
    }

    /// Whether every rule the kill bar names ran at block level. The run reads
    /// no `weeder.toml` of its own, so this asks the catalogue and the defaults the
    /// binary ships, and the report writes the answer down.
    fn load_bearing_intact(&self) -> bool {
        let config = Config::default();
        LOAD_BEARING
            .iter()
            .all(|rule| level_of(rule, &config) == Some(Level::Block))
    }

    pub fn outcome(&self) -> Outcome {
        let judged = self.judged();
        let tally = self.pooled();
        let share = share(tally.against_the_bar(), judged);
        Outcome {
            judged,
            blocked: tally.blocked(),
            tally,
            refusals: self.refusals(),
            ships: share < BAR && self.refusals() == 0 && self.load_bearing_intact(),
            floor: self.floor(),
        }
    }

    /// The verdict's floor: the blocked commits that count against the bar when
    /// the second party is believed over the ledger wherever the two disagree.
    ///
    /// A block counts if the ledger called it a false positive, if nobody
    /// classified it, or if a blind re-grade called it one. Nothing here reads
    /// the sighted re-grade: an auditor who saw the class before judging hands
    /// the builder's own answer back, and a floor built from that would move
    /// nothing.
    fn floor(&self) -> usize {
        let regraded = self.audit.blind_false_positives();
        let mut count = 0;
        for repo in self.repos {
            for blocked in &repo.blocked {
                let ledger = self
                    .ledger
                    .get(&repo.name, &blocked.sha)
                    .map(|judgement| judgement.classification);
                let against = matches!(ledger, None | Some(Classification::FalsePositive));
                if against
                    || regraded
                        .iter()
                        .any(|row| row.names(&repo.name, &blocked.sha))
                {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let outcome = self.outcome();
        out.push_str("# calibration: weeder over real history, 2026-09\n\n");
        out.push_str(&self.verdict(&outcome));
        out.push_str(&self.method());
        out.push_str(&self.levels());
        for repo in self.repos {
            out.push_str(&self.repo_section(repo));
        }
        out.push_str(&self.totals(&outcome));
        out.push_str(&self.moved());
        out.push_str(&self.wrong());
        out.push_str(&self.precision());
        out.push_str(&self.rule_view());
        out
    }

    /// The first paragraph. The verdict first, ship or kill, with the number
    /// that decided it and the qualification the independent re-grade has or has
    /// not lifted; then, in the same breath, whether the re-grade the number
    /// rests on was taken sighted or blind. Both are written from the audit
    /// files rather than typed, so a verdict cannot be promoted, and an
    /// agreement cannot be made to look blinder than it was, by editing this
    /// report.
    fn verdict(&self, outcome: &Outcome) -> String {
        let share = share(outcome.tally.against_the_bar(), outcome.judged);
        let repos = self.repos.len();
        let mut verdict = String::new();
        if outcome.ships {
            let _ = write!(
                verdict,
                "weeder ships as a gate{}: over {} commits of real history in {repos} repositories it blocked {}, of which {} were block-level false positives, {share:.2} percent of the commits judged and under the two percent bar, with {} all still at block level{}. {}\n\n",
                if self.audit.confirms() {
                    String::new()
                } else {
                    ", pending the independent re-grade".to_string()
                },
                outcome.judged,
                outcome.blocked,
                outcome.tally.against_the_bar(),
                spell(&LOAD_BEARING),
                // The word the audit loop's evidence looks for stays inside the
                // first sentence, so a reader who stops there has read it.
                if self.audit.confirms() {
                    ""
                } else {
                    ", and the classification under it untrusted until a re-grade agrees at the bar"
                },
                self.audit.sight(),
            );
            let mut after = match (self.audit.pending(), &self.audit.samples) {
                (Some(pending), _) => format!(
                    "The classification under that number is the builder's own, and {pending}. The verdict stands as provisional until an agreement of {:.0} percent or better is recorded on every sample the re-grade draws. `cargo xtask calibrate` writes this sentence from that file, so editing this one changes nothing.",
                    crate::audit::AGREEMENT_BAR,
                ),
                (None, Some(samples)) => {
                    let read: Vec<String> = samples
                        .iter()
                        .map(|sample| {
                            format!(
                                "{} at {:.1} percent of {} cases",
                                sample.name,
                                sample.agreement(),
                                sample.regraded
                            )
                        })
                        .collect();
                    format!(
                        "A second party re-graded the classification and {} records the agreement: {}. That is what took the qualification off this sentence.",
                        self.audit.label,
                        read.join(", "),
                    )
                }
                (None, None) => String::new(),
            };
            if !after.is_empty() {
                after.push(' ');
            }
            after.push_str(&self.floor_sentence(outcome));
            let _ = write!(verdict, "{after}\n\n");
        } else {
            let _ = write!(
                verdict,
                "weeder does not ship as a gate: over {} commits of real history in {repos} repositories it blocked {}, of which {} were block-level false positives, {share:.2} percent of the commits judged against a two percent bar{}. {}\n\n",
                outcome.judged,
                outcome.blocked,
                outcome.tally.against_the_bar(),
                if outcome.refusals > 0 {
                    format!(", and {} commits it could not judge at all", outcome.refusals)
                } else {
                    String::new()
                },
                self.audit.sight(),
            );
            let _ = write!(verdict, "{}\n\n", self.floor_sentence(outcome));
        }
        if !self.load_bearing_intact() {
            let _ = write!(
                verdict,
                "The kill bar was reached: the run did not judge with {} at block level, so whatever the share says, weeder does not ship as a gate.\n\n",
                spell(&LOAD_BEARING)
            );
        }
        verdict
    }

    /// The verdict's floor, in the paragraph after the verdict: what the pooled
    /// block-level false-positive share becomes when every `false-positive` the
    /// blind re-grade recorded is taken as true, printed beside the ledger's own
    /// share.
    ///
    /// A reader who is handed one share has to take the classification behind it
    /// on trust. Handed both, they can see how far the number can move under the
    /// harshest reading anybody has written down, and decide on that. It is read
    /// off the blind re-grade's own table, so a re-grade that reads more blocks
    /// back moves this number by being written.
    fn floor_sentence(&self, outcome: &Outcome) -> String {
        let floor = self.floor_only(outcome);
        match self.ruling {
            Some(ruling) => format!("{floor} {}", ruling.sentence()),
            None => floor,
        }
    }

    fn floor_only(&self, outcome: &Outcome) -> String {
        let ledger = share(outcome.tally.against_the_bar(), outcome.judged);
        let floor = share(outcome.floor, outcome.judged);
        if !self.audit.blind_regraded_blocks() {
            return format!(
                "No blind re-grade has read a blocked commit back, so the floor under that share is the ledger's own {ledger:.2} percent, {} of {} commits judged.",
                outcome.tally.against_the_bar(),
                outcome.judged,
            );
        }
        format!(
            "The floor under that share is {floor:.2} percent, {} of the {} commits judged, with every `false-positive` verdict the blind re-grade in {} recorded taken as true; the ledger's own reading is {ledger:.2} percent, {} of the same {}. The floor is the harshest reading this file records, and `cargo xtask calibrate` draws it from that re-grade's table.",
            outcome.floor,
            outcome.judged,
            self.audit.blind_labels(),
            outcome.tally.against_the_bar(),
            outcome.judged,
        )
    }

    fn method(&self) -> String {
        let mut out = String::from("## How this was measured\n\n");
        out.push_str(&format!(
            "`cargo xtask calibrate` takes the last {WINDOW} commits ending at the commit `docs/calibration/corpus.toml` pins each repository at, and judges each one against its first parent with the same `check` face the binary runs: `weeder check --base <parent> --strict`, read back as SARIF. A history with fewer commits contributes all of them, and a repository judged on fewer than {REPORTED_ONLY} commits is reported rather than judged on its own share.\n\n",
        ));
        out.push_str("The corpus names each repository by a source `git fetch` can read and by the full sha its window ends at. The pin is what a reader can hold this file to: a commit pushed to any of these repositories after the pin falls outside the window and cannot move a number here, and the same corpus judges the same history on a machine that has never seen any of these repositories. Moving a pin is an edit to that file, and the run that follows it is a new measurement.\n\n");
        out.push_str("A merge commit is left out of the window. It carries no change of its own, and the commits it brings are in the same window, so judging it as well would weigh one change twice. A root commit is left out too: this measurement judges commits against their parents, and a root has none.\n\n");
        out.push_str("Nothing is written to the repositories being read. Each pinned commit and everything it reaches is fetched into a scratch repository under the system's temp directory, every checkout and every judgement happens there, and the scratch is removed at the end. A fetch runs `upload-pack` at the source, which hands objects out and takes none in. Where a source is a directory on the machine running the measurement, its refs, HEAD and working tree are fingerprinted before the run and again after it, and a difference stops the run.\n\n");
        out.push_str("A block is classified by whoever ran the calibration, in `docs/calibration/judgements.toml`, one line of reasoning per commit. The line between the three classes is what the finding claims, not how welcome it was: **true positive**, the claim is true and the change really did weaken something; **acceptable**, the claim is true and the change was fine anyway, so the block is friction the rule was designed to create; **false positive**, the claim is not true of this change. A blocked commit nobody has classified counts as a false positive everywhere a number is drawn, so the bar can only be reached by reading the diffs. `scripts/check/calibration-bar.sh` checks the arithmetic and the bar, never the judgement.\n\n");
        out
    }

    /// The levels the run judged at, so the kill bar can be read off the file
    /// rather than taken on trust.
    fn levels(&self) -> String {
        let config = Config::default();
        let mut out = String::from("## The rules that ran\n\n");
        out.push_str("No `weeder.toml` was passed and none was read: every rule ran at the level the catalogue ships it at. The four rules the kill bar names are first.\n\n");
        out.push_str("| Rule | Level in this run | What it finds |\n|---|---|---|\n");
        let mut rules: Vec<_> = catalogue::rules()
            .iter()
            .filter(|rule| rule.face == Face::Check)
            .collect();
        rules.sort_by_key(|rule| (!LOAD_BEARING.contains(&rule.id), rule.id));
        for rule in rules {
            let _ = writeln!(
                out,
                "| {} | {} | {} |",
                rule.id,
                spell_level(level_of(rule.id, &config)),
                cell(rule.short_description),
            );
        }
        out.push('\n');
        let carried: Vec<String> = self
            .repos
            .iter()
            .filter(|repo| repo.configured > 0)
            .map(|repo| format!("{} ({} commits)", repo.name, repo.configured))
            .collect();
        if carried.is_empty() {
            out.push_str("No commit in the corpus carried a `weeder.toml` of its own, so no repository moved a rule off the level above.\n\n");
        } else {
            let _ = writeln!(
                out,
                "These commits carried a `weeder.toml` of their own, which weeder read as the repository's law: {}. Their levels are whatever those files say.\n",
                carried.join(", ")
            );
        }
        out
    }

    fn repo_section(&self, repo: &RepoMeasurement) -> String {
        let tally = self.tally(repo);
        let mut out = format!(
            "## {}, {} commits judged, {} blocked, {} warned\n\n",
            repo.name,
            repo.judged,
            tally.blocked(),
            repo.warned
        );
        let _ = writeln!(
            out,
            "The window ends at `{}`, fetched from `{}`: the last {} commits reaching it that are not merges, {} of which have a parent to be judged against.{}\n",
            repo.tip,
            repo.source,
            repo.window,
            repo.judged,
            if repo.judged_alone() {
                String::new()
            } else {
                format!(
                    " Fewer than {REPORTED_ONLY} commits were judged, so this repository is reported and not judged on its own share."
                )
            }
        );
        if !repo.refusals.is_empty() {
            out.push_str("weeder could not judge these commits at all, which is a failure of the run and not a pass:\n\n");
            for refusal in &repo.refusals {
                let _ = writeln!(
                    out,
                    "- `{}`, {}",
                    short(&refusal.sha),
                    cell(&refusal.reason)
                );
            }
            out.push('\n');
        }
        if repo.blocked.is_empty() {
            out.push_str("Nothing blocked.\n\n");
            return out;
        }
        out.push_str(
            "| Commit | Rules at block level | Classification | Why |\n|---|---|---|---|\n",
        );
        for blocked in &repo.blocked {
            let judgement = self.ledger.get(&repo.name, &blocked.sha);
            let _ = writeln!(
                out,
                "| `{}` {} | {} | {} | {} |",
                short(&blocked.sha),
                cell(&blocked.subject),
                blocked.rules.join(", "),
                judgement
                    .map(|judgement| judgement.classification.spelled())
                    .unwrap_or("unclassified"),
                judgement
                    .map(|judgement| cell(&judgement.reasoning))
                    .unwrap_or_else(|| "nobody has read this block yet; it counts as a false positive until somebody does".to_string()),
            );
        }
        out.push('\n');
        out
    }

    fn totals(&self, outcome: &Outcome) -> String {
        let mut out = String::from("## Totals\n\n");
        out.push_str("| Repo | Commits judged | Blocked | Warned | True positive | Acceptable | False positive | False-positive share |\n|---|---|---|---|---|---|---|---|\n");
        for repo in self.repos {
            let tally = self.tally(repo);
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {:.2}% |",
                repo.name,
                repo.judged,
                tally.blocked(),
                repo.warned,
                tally.true_positive,
                tally.acceptable,
                tally.against_the_bar(),
                share(tally.against_the_bar(), repo.judged),
            );
        }
        let pooled = outcome.tally;
        let _ = writeln!(
            out,
            "| **pooled** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{:.2}%** |",
            outcome.judged,
            pooled.blocked(),
            self.repos.iter().map(|repo| repo.warned).sum::<usize>(),
            pooled.true_positive,
            pooled.acceptable,
            pooled.against_the_bar(),
            share(pooled.against_the_bar(), outcome.judged),
        );
        out.push('\n');
        let _ = writeln!(
            out,
            "The bar is a pooled block-level false-positive share under {BAR:.0} percent of the commits judged. This run is at {:.2} percent of {} commits, and {} commits weeder could not judge.\n",
            share(pooled.against_the_bar(), outcome.judged),
            outcome.judged,
            outcome.refusals,
        );
        out.push_str(&self.split_summary());
        let over: Vec<String> = self
            .repos
            .iter()
            .filter(|repo| repo.judged_alone())
            .filter(|repo| share(self.tally(repo).against_the_bar(), repo.judged) >= BAR)
            .map(|repo| {
                format!(
                    "{} at {:.2} percent",
                    repo.name,
                    share(self.tally(repo).against_the_bar(), repo.judged)
                )
            })
            .collect();
        if !over.is_empty() {
            let _ = writeln!(
                out,
                "Read on their own these repositories are over the bar: {}. The bar is pooled, and they are named here rather than left to be found in the table.\n",
                over.join(", ")
            );
        }
        let small: Vec<String> = self
            .repos
            .iter()
            .filter(|repo| !repo.judged_alone())
            .map(|repo| {
                format!(
                    "{} ({} commits, {:.2} percent)",
                    repo.name,
                    repo.judged,
                    share(self.tally(repo).against_the_bar(), repo.judged)
                )
            })
            .collect();
        if !small.is_empty() {
            let _ = writeln!(
                out,
                "Too few commits to carry a share of their own, reported and not judged alone: {}. Their commits and their false positives are both in the pooled total.\n",
                small.join(", ")
            );
        }
        out
    }

    /// What the rules moved since the first run, in one paragraph beside the
    /// pooled share. Every count here is a row of the section below it, so a
    /// reader who does not believe the sentence can add the table up.
    fn split_summary(&self) -> String {
        let split = self.split;
        if split.rows.is_empty() && split.unjudged.is_empty() {
            return "No earlier run's blocks are recorded against this corpus, so there is nothing here to compare this one with.\n\n".to_string();
        }
        let mut out = String::new();
        let _ = write!(
            out,
            "Beside that share, what the rules moved. The first run refused {} findings at block level on these commits, recorded in `{}`. This run reports {} of them at block level still, {} at warn level, {} at note level and {} not at all.",
            split.blocked_then(),
            split.record,
            split.at(split::Level::Block),
            split.at(split::Level::Warn),
            split.at(split::Level::Note),
            split.at(split::Level::Silent),
        );
        let moved = split.moves(split::Level::Warn);
        if moved.is_empty() {
            out.push_str(" Nothing dropped from block level to warn level.");
        } else {
            let spelled: Vec<String> = moved
                .iter()
                .map(|moved| {
                    format!(
                        "{} moved from {} at block level to {} at warn level, {}",
                        moved.count,
                        moved.then,
                        moved.now,
                        spell_kinds(&moved.kinds),
                    )
                })
                .collect();
            let _ = write!(out, " Rule by rule: {}.", spelled.join("; "));
        }
        if !split.unjudged.is_empty() {
            let _ = write!(
                out,
                " {} of the first run's block-level findings sit on commits outside this window and are not compared.",
                split.unjudged.len()
            );
        }
        out.push_str("\n\n");
        out
    }

    /// One row per finding the first run refused, and what this run says about
    /// the same file in the same commit. The paragraph above is this table added
    /// up, and the evidence script adds it up again.
    fn moved(&self) -> String {
        let split = self.split;
        if split.rows.is_empty() && split.unjudged.is_empty() {
            return String::new();
        }
        let mut out = String::from("## What the rules moved since the first run\n\n");
        out.push_str("A rule that splits in two is supposed to take friction off the gate while keeping the finding. Until the same files are read twice that is a claim. Every file whose answer changed is here with the rule that refused it, the kind weeder gives the file, and the loudest thing weeder says about it now.\n\n");
        let _ = writeln!(
            out,
            "{} of the first run's block-level findings are refused by the same rule at the same level and are left out of this table; the {} whose answer changed are all in it.\n",
            split.unchanged(),
            split.changed().count(),
        );
        out.push_str("| Repo | Commit | File | Kind | Blocked then by | Says now | At |\n|---|---|---|---|---|---|---|\n");
        for row in split.changed() {
            let _ = writeln!(
                out,
                "| {} | `{}` | `{}` | {} | {} | {} | {} |",
                row.repo,
                short(&row.commit),
                cell(&row.path),
                row.kind,
                row.then,
                row.now.as_deref().unwrap_or("nothing"),
                row.level.spelled(),
            );
        }
        out.push('\n');
        if !split.unjudged.is_empty() {
            out.push_str("And these findings of the first run sit on commits this window does not reach, so this run says nothing about them:\n\n");
            for block in &split.unjudged {
                let _ = writeln!(
                    out,
                    "- {} `{}` `{}`, {}",
                    block.repo,
                    short(&block.commit),
                    cell(&block.path),
                    block.rule,
                );
            }
            out.push('\n');
        }
        out
    }

    /// Every block whose claim was not true, grouped by the rule that made it.
    /// A calibration that only prints a share hides the thing worth acting on,
    /// which is the shape of the mistake and where to find it again.
    fn wrong(&self) -> String {
        let mut by_rule: BTreeMap<&str, Vec<(&str, &crate::calibrate::Blocked, &str)>> =
            BTreeMap::new();
        let mut unread: Vec<(&str, &crate::calibrate::Blocked)> = Vec::new();
        for repo in self.repos {
            for blocked in &repo.blocked {
                let Some(judgement) = self.ledger.get(&repo.name, &blocked.sha) else {
                    unread.push((repo.name.as_str(), blocked));
                    continue;
                };
                for rule in &blocked.rules {
                    if judgement.for_rule(rule) == Classification::FalsePositive {
                        by_rule.entry(rule.as_str()).or_default().push((
                            repo.name.as_str(),
                            blocked,
                            judgement.reason_for_rule(rule),
                        ));
                    }
                }
            }
        }

        let mut out = String::from("## Where weeder was wrong\n\n");
        if by_rule.is_empty() && unread.is_empty() {
            out.push_str("No block's claim turned out to be untrue.\n\n");
            return out;
        }
        out.push_str("Every block whose claim was not true of the change, under the rule that made it. This is the list the rule loops work from.\n\n");
        for (rule, blocks) in &by_rule {
            let _ = writeln!(
                out,
                "**{rule}**, {}\n",
                if blocks.len() == 1 {
                    "one block".to_string()
                } else {
                    format!("{} blocks", blocks.len())
                }
            );
            for (repo, blocked, reasoning) in blocks {
                let _ = writeln!(
                    out,
                    "- {repo} `{}` {}, {}",
                    short(&blocked.sha),
                    cell(&blocked.subject),
                    cell(reasoning),
                );
            }
            out.push('\n');
        }
        if !unread.is_empty() {
            out.push_str("And these blocks nobody has read yet, which count as false positives until somebody does:\n\n");
            for (repo, blocked) in &unread {
                let _ = writeln!(
                    out,
                    "- {repo} `{}` {}",
                    short(&blocked.sha),
                    cell(&blocked.subject)
                );
            }
            out.push('\n');
        }
        out
    }

    fn precision(&self) -> String {
        let mut out = String::from("## Precision and the allowance rate\n\n");
        out.push_str("Precision is the share of blocks that were not false positives. The allowance rate beside it counts `Weed-allow:` trailers per hundred commits from the day the repository installed guard, and is zero before that day because there was no gate to allow anything past. Allowances rising while true positives stay flat is a gate being routed around rather than obeyed.\n\n");
        out.push_str("| Repo | Block-level precision | True positives | Allowance rate |\n|---|---|---|---|\n");
        for repo in self.repos {
            let tally = self.tally(repo);
            let rate = self
                .rates
                .iter()
                .find(|rate| rate.name == repo.name)
                .map(RepoRate::spelled)
                .unwrap_or_else(|| "not measured".to_string());
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                repo.name,
                precision(&tally),
                tally.true_positive,
                rate,
            );
        }
        out.push('\n');
        let stray: Vec<&RepoRate> = self
            .rates
            .iter()
            .filter(|rate| rate.trailers_before > 0)
            .collect();
        if stray.is_empty() {
            out.push_str("No repository wrote an allowance before it installed guard.\n\n");
        } else {
            for rate in stray {
                let _ = writeln!(
                    out,
                    "{} carries {} `Weed-allow:` trailers written before guard was installed. They are outside the rate and worth reading: an allowance against a gate that was not running yet allows nothing.\n",
                    rate.name, rate.trailers_before
                );
            }
        }
        out
    }

    fn rule_view(&self) -> String {
        let mut fires: BTreeMap<&str, Tally> = BTreeMap::new();
        for repo in self.repos {
            for blocked in &repo.blocked {
                let judgement = self.ledger.get(&repo.name, &blocked.sha);
                for rule in &blocked.rules {
                    fires
                        .entry(rule.as_str())
                        .or_default()
                        .add(judgement.map(|judgement| judgement.for_rule(rule)));
                }
            }
        }
        let mut out = String::from("## The rules that blocked\n\n");
        out.push_str("A commit that fired two rules is counted once under each. Where one rule of a block deserves a different answer from the commit as a whole, the ledger says so and this table follows it.\n\n");
        out.push_str("| Rule | Blocks | True positive | Acceptable | False positive | False-positive share of its blocks |\n|---|---|---|---|---|---|\n");
        for (rule, tally) in &fires {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {:.2}% |",
                rule,
                tally.blocked(),
                tally.true_positive,
                tally.acceptable,
                tally.against_the_bar(),
                share(tally.against_the_bar(), tally.blocked()),
            );
        }
        if fires.is_empty() {
            out.push_str("| none | 0 | 0 | 0 | 0 | 0.00% |\n");
        }
        out.push('\n');
        out
    }
}

fn precision(tally: &Tally) -> String {
    if tally.blocked() == 0 {
        return "no blocks".to_string();
    }
    format!(
        "{:.1}%",
        (tally.blocked() - tally.against_the_bar()) as f64 * 100.0 / tally.blocked() as f64
    )
}

/// A percentage that is zero rather than undefined when there is nothing to
/// divide by: no commits judged is no false positives, and the report says the
/// count beside it.
pub fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}

/// The level a rule reports at under a config, asked of the same function the
/// binary asks. A rule the catalogue does not know has no level to report at.
fn level_of(id: &str, config: &Config) -> Option<Level> {
    catalogue::rule(id).and_then(|rule| configured_level(rule, config))
}

fn spell_level(level: Option<Level>) -> &'static str {
    match level {
        Some(Level::Block) => "block",
        Some(Level::Warn) => "warn",
        Some(Level::Note) => "note",
        None => "not run",
    }
}

/// The kinds of file a group of findings sits on, counted.
fn spell_kinds(kinds: &[(String, usize)]) -> String {
    match kinds {
        [] => "on no file weeder could classify".to_string(),
        [(kind, _)] => format!("all of them on {kind} files"),
        _ => {
            let spelled: Vec<String> = kinds
                .iter()
                .map(|(kind, count)| format!("{count} on {kind} files"))
                .collect();
            spelled.join(" and ")
        }
    }
}

/// A list of rule ids as a sentence reads them.
fn spell(rules: &[&str]) -> String {
    match rules {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn short(sha: &str) -> String {
    sha.chars().take(10).collect()
}

/// Text safe to put in a markdown table cell: a pipe would end the cell, and a
/// newline would end the row.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}
