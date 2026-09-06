//! The independent re-grade, and what the verdict may say because of it.
//!
//! Whether a block was a false positive is a judgement, and the builder's own
//! judgement is not the number weeder ships on. A second party re-classifies a
//! sample blind and writes `docs/calibration-audit-2026-09.md`; until that file
//! records agreement at or above ninety percent on every sample it drew, the
//! calibration's first sentence says the verdict is pending.
//!
//! The wording is therefore never an edit. The generator reads the audit file
//! and writes the sentence the file earns, so nobody can promote a provisional
//! verdict by retyping it.
//!
//! The file is markdown, written by whoever did the re-grade, and one table in
//! it is read: a row per sample, with how many cases were re-graded and how many
//! of those agreed. The share is recomputed from those two numbers rather than
//! read out of the column beside them.
//!
//! One more line of it is read, and it decides the sentence after the verdict:
//! the `Blind:` declaration each re-grade makes about itself. An auditor who
//! could see the class beside each case before judging agreed with something in
//! front of them, and a report that prints the agreement without saying so
//! prints a stronger number than it has.

use std::path::{Path, PathBuf};

/// The agreement every sample has to reach for the verdict to stand unqualified.
pub const AGREEMENT_BAR: f64 = 90.0;

/// The smallest sample the verdict may rest on. The audit loop draws at least
/// twenty of each; calibration writes the sentence, so calibration checks that
/// the sample under it is the one that was asked for.
pub const SAMPLE_FLOOR: usize = 20;

/// One sample the re-grade drew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    pub name: String,
    pub regraded: usize,
    pub agreed: usize,
}

impl Sample {
    pub fn agreement(&self) -> f64 {
        if self.regraded == 0 {
            return 0.0;
        }
        self.agreed as f64 * 100.0 / self.regraded as f64
    }

    /// Whether one sample on its own is enough to rest a verdict on.
    pub fn stands(&self) -> bool {
        self.regraded >= SAMPLE_FLOOR
            && self.agreed <= self.regraded
            && self.agreement() >= AGREEMENT_BAR
    }
}

/// One row of a re-grade's blocked commit sample: the case it re-graded and the
/// class it gave that case, in its own words.
///
/// The verdict is kept as the file spells it, lowercased and hyphenated, because
/// a re-grade may write a word this repository has no class for and the report
/// would rather say so than round it to one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regraded {
    pub repo: String,
    /// The commit as the re-grade names it, which is usually an abbreviation.
    pub sha: String,
    pub verdict: String,
}

impl Regraded {
    /// Whether this row is about the blocked commit a measurement holds. The
    /// re-grade abbreviates the sha and the measurement does not, so the shorter
    /// of the two has to be a prefix of the longer.
    pub fn names(&self, repo: &str, sha: &str) -> bool {
        self.repo == repo && (sha.starts_with(&self.sha) || self.sha.starts_with(sha))
    }
}

/// Whether the auditor could see the builder's classification while judging.
///
/// It is read from the file's own `Blind:` declaration and from nothing else:
/// not from the file's name, not from which run wrote it. A re-grade that says
/// nothing about it has declared nothing, and the report says so rather than
/// guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sight {
    /// `Blind: yes`. The auditor judged from a packet, with the builder's class
    /// out of sight.
    Blind,
    /// `Blind: no`. The auditor could read the class beside each case.
    Sighted,
    /// The file carries no `Blind:` line.
    Undeclared,
}

/// One re-grade the repository carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The file as the report names it.
    pub label: String,
    pub sight: Sight,
    pub samples: Vec<Sample>,
    /// Every row of this re-grade's blocked commit sample, as it wrote them.
    pub blocked: Vec<Regraded>,
}

impl Record {
    /// Whether this re-grade on its own agrees at the bar on every sample it
    /// drew. The pooled verdict asks the same of all of them together; the
    /// anchoring caveat asks it of each, because one re-grade being blind is
    /// only worth saying while that re-grade stands.
    pub fn stands(&self) -> bool {
        !self.samples.is_empty() && self.samples.iter().all(Sample::stands)
    }
}

/// The re-grade, and why the verdict does or does not rest on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Audit {
    /// The file as the report names it: a path inside the repository, never one
    /// that only exists on the machine the run happened on.
    pub label: String,
    /// Nothing where the file is not there yet.
    pub samples: Option<Vec<Sample>>,
    /// One per re-grade file that is there, in the order they were read. The
    /// pooled `samples` above decide the verdict; these decide what the report
    /// may say about how the re-grade was taken.
    pub records: Vec<Record>,
}

impl Audit {
    /// Whether the verdict may drop the qualification: the file is there, it
    /// drew at least one sample, and every sample it drew agrees at the bar.
    pub fn confirms(&self) -> bool {
        match &self.samples {
            Some(samples) => !samples.is_empty() && samples.iter().all(Sample::stands),
            None => false,
        }
    }

    /// Why the verdict is still pending, in the words the report prints. Nothing
    /// where it is not.
    pub fn pending(&self) -> Option<String> {
        if self.confirms() {
            return None;
        }
        let Some(samples) = &self.samples else {
            return Some(format!(
                "{} is not written yet, so nobody outside this run has read the classifications back",
                self.label
            ));
        };
        if samples.is_empty() {
            return Some(format!(
                "{} draws no sample, so there is no agreement to read off it",
                self.label
            ));
        }
        let short: Vec<String> = samples
            .iter()
            .filter(|sample| !sample.stands())
            .map(|sample| {
                if sample.regraded < SAMPLE_FLOOR {
                    format!(
                        "{} is {} cases and the sample floor is {SAMPLE_FLOOR}",
                        sample.name, sample.regraded
                    )
                } else {
                    format!(
                        "{} agrees on {:.1} percent of {} cases",
                        sample.name,
                        sample.agreement(),
                        sample.regraded
                    )
                }
            })
            .collect();
        Some(format!(
            "the re-grade is under the {AGREEMENT_BAR:.0} percent bar: {}",
            short.join(", ")
        ))
    }

    /// What stands behind the classification, in the words the report prints in
    /// its first paragraph right after the verdict: whether the re-grade the
    /// number rests on was taken with the builder's class in sight or without
    /// it. An agreement read off a file that showed the auditor the class it
    /// was agreeing with is anchored to that class, and forty of forty reads
    /// stronger than it is when nobody says so.
    ///
    /// Every word of it is read from the files' own `Blind:` declarations, so a
    /// re-grade redone blind changes this sentence by being written and never by
    /// being described.
    pub fn sight(&self) -> String {
        if self.records.is_empty() {
            return "No re-grade is recorded, so no agreement stands behind that classification to call sighted or blind.".to_string();
        }
        let of = |wanted: Sight| -> Vec<&Record> {
            self.records
                .iter()
                .filter(|record| record.sight == wanted)
                .collect()
        };
        let blind = of(Sight::Blind);
        let sighted = of(Sight::Sighted);
        let undeclared = of(Sight::Undeclared);
        let standing: Vec<&Record> = blind
            .iter()
            .copied()
            .filter(|record| record.stands())
            .collect();
        if !standing.is_empty() {
            return format!(
                "The re-grade behind it is blind: {} declares `Blind: yes` and agrees at the bar, so its auditor judged the cases from a packet with the builder's class out of sight, and {}.",
                named(&standing),
                if sighted.is_empty() {
                    "no sighted re-grade is recorded beside it".to_string()
                } else {
                    format!(
                        "the sighted re-grade in {} is recorded beside it",
                        named(&sighted)
                    )
                },
            );
        }
        let short: Vec<&Record> = blind
            .iter()
            .copied()
            .filter(|record| !record.stands())
            .collect();
        let mut parts = Vec::new();
        if !sighted.is_empty() {
            parts.push(format!(
                "The re-grade behind it is a sighted one: {} declares `Blind: no`, so its auditor could read the builder's class beside each case before judging.",
                named(&sighted)
            ));
        }
        if !short.is_empty() {
            parts.push(format!(
                "The blind re-grade in {} is under the bar, so no blind agreement stands behind this number yet.",
                named(&short)
            ));
        }
        if !undeclared.is_empty() {
            parts.push(format!(
                "{} declares no `Blind:` line either way, so what its auditor could read while judging is unrecorded.",
                named(&undeclared)
            ));
        }
        parts.join(" ")
    }

    /// Every `false-positive` a blind re-grade recorded against a blocked
    /// commit. It is what the verdict's floor is drawn from: the harshest
    /// reading the repository has written down, with the second party believed
    /// over the ledger wherever the two disagree.
    ///
    /// Only a re-grade that declared itself blind counts. A sighted auditor read
    /// the class before judging, so a sighted `false-positive` is the builder's
    /// own answer coming back, and a floor built out of it would say nothing.
    pub fn blind_false_positives(&self) -> Vec<&Regraded> {
        self.records
            .iter()
            .filter(|record| record.sight == Sight::Blind)
            .flat_map(|record| record.blocked.iter())
            .filter(|row| row.verdict == "false-positive")
            .collect()
    }

    /// Whether any blind re-grade re-graded a blocked commit at all. A floor
    /// drawn from no rows is the ledger's own share, and the report says which
    /// of the two it is printing.
    pub fn blind_regraded_blocks(&self) -> bool {
        self.records
            .iter()
            .any(|record| record.sight == Sight::Blind && !record.blocked.is_empty())
    }

    /// The blind re-grades the floor was drawn from, as the report names them.
    pub fn blind_labels(&self) -> String {
        let blind: Vec<&Record> = self
            .records
            .iter()
            .filter(|record| record.sight == Sight::Blind && !record.blocked.is_empty())
            .collect();
        named(&blind)
    }
}

/// The files behind a list of re-grades, as the report names them.
fn named(records: &[&Record]) -> String {
    records
        .iter()
        .map(|record| record.label.clone())
        .collect::<Vec<String>>()
        .join(" and ")
}

/// The audit a run reads when no other path is given.
pub const DEFAULT_PATH: &str = "docs/calibration-audit-2026-09.md";

/// Whether a file under docs/ is an audit's own record rather than the packet,
/// response or transcript kept beside it.
fn is_audit_record(name: &str) -> bool {
    name.starts_with("calibration-audit")
        && name.ends_with(".md")
        && !name.contains(".packet.")
        && !name.contains(".response.")
        && !name.contains("transcript")
}

/// Every re-grade the repository carries, read as one: the verdict drops its
/// qualification only when every audit file agrees at the bar on every sample,
/// so a blind re-grade under the bar keeps the verdict pending however well a
/// sighted one did. Each sample is named by the file it came from.
pub fn read_all(root: &Path) -> Audit {
    let docs = root.join("docs");
    let mut names: Vec<String> = std::fs::read_dir(&docs)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| is_audit_record(name))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    if names.is_empty() {
        return Audit {
            label: DEFAULT_PATH.to_string(),
            samples: None,
            records: Vec::new(),
        };
    }
    let files: Vec<(PathBuf, String)> = names
        .iter()
        .map(|name| (docs.join(name), format!("docs/{name}")))
        .collect();
    gather(&files, true)
}

/// Read the re-grades a caller names, which is how a suite probes the wording
/// against files it wrote itself. A file that is not there is not an error: the
/// re-grade is work that happens after the calibration, and its absence is
/// exactly what the provisional wording is for.
pub fn read_many(files: &[(PathBuf, String)]) -> Audit {
    gather(files, false)
}

/// The files, read into the one audit the report is written from. Samples are
/// named by the file they came from only where more than one file was read for
/// the repository's own record, because a reader of the shipped report has to
/// know which re-grade a share came off.
fn gather(files: &[(PathBuf, String)], name_samples_by_file: bool) -> Audit {
    let label = files
        .iter()
        .map(|(_, label)| label.clone())
        .collect::<Vec<String>>()
        .join(" and ");
    let mut records = Vec::new();
    for (path, label) in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        records.push(Record {
            label: label.clone(),
            sight: declared_sight(&text),
            samples: samples(&text),
            blocked: blocked_rows(&text),
        });
    }
    if records.is_empty() {
        return Audit {
            label,
            samples: None,
            records,
        };
    }
    // Ruling of 2026-09-06: only a blind re-grade can lift the qualification. A
    // sighted one, where the auditor could read the builder's class, is kept in
    // `records` and named beside the verdict, and never pooled into what
    // decides it.
    let mut all = Vec::new();
    let mut counted = 0;
    for record in &records {
        if !matches!(record.sight, Sight::Blind) {
            continue;
        }
        counted += 1;
        for sample in &record.samples {
            let mut sample = sample.clone();
            if name_samples_by_file {
                sample.name = format!("{} in {}", sample.name, record.label);
            }
            all.push(sample);
        }
    }
    if counted == 0 {
        return Audit {
            label: "a blind re-grade".to_string(),
            samples: None,
            records,
        };
    }
    Audit {
        label,
        samples: Some(all),
        records,
    }
}

/// The file's own declaration of how the re-grade was taken: a line reading
/// `Blind: yes` or `Blind: no`, with whatever the writer put after a semicolon
/// or a comma as the reason for it. Emphasis and heading marks around the line
/// are ignored, and a line whose answer is neither word declares nothing.
fn declared_sight(text: &str) -> Sight {
    for line in text.lines() {
        let plain: String = line
            .chars()
            .filter(|mark| !matches!(mark, '*' | '_' | '#' | '>' | '`'))
            .collect();
        let Some(answer) = plain
            .trim()
            .to_ascii_lowercase()
            .strip_prefix("blind:")
            .map(|rest| {
                rest.split([';', ',', '.'])
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
        else {
            continue;
        };
        match answer.as_str() {
            "yes" | "true" => return Sight::Blind,
            "no" | "false" => return Sight::Sighted,
            _ => continue,
        }
    }
    Sight::Undeclared
}

/// The rows of the file's blocked commit sample: `| <repo> | <commit> |
/// <verdict> | <reasoning> |`, under the heading that names it.
///
/// The heading is what bounds it, because the same file carries a recall sample
/// whose rows are shaped differently and an agreement table whose rows are
/// numbers. Inside it, a row counts when its second cell is a commit: that is
/// what leaves the header and the rule under it where they are.
fn blocked_rows(text: &str) -> Vec<Regraded> {
    let mut rows = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            inside = heading.to_ascii_lowercase().contains("blocked commit");
            continue;
        }
        if !inside || !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 3 {
            continue;
        }
        let repo = plain(cells[0]);
        let sha = plain(cells[1]);
        let verdict = plain(cells[2]).to_ascii_lowercase().replace(' ', "-");
        if repo.is_empty() || sha.is_empty() || verdict.is_empty() {
            continue;
        }
        if !sha.chars().all(|mark| mark.is_ascii_hexdigit()) {
            continue;
        }
        rows.push(Regraded { repo, sha, verdict });
    }
    rows
}

/// One table cell without the marks markdown puts around a value.
fn plain(cell: &str) -> String {
    cell.trim()
        .trim_matches('`')
        .trim_matches('*')
        .trim()
        .to_string()
}

/// The samples in the agreement table: `| <sample> | <re-graded> | <agreed> |`,
/// and whatever else the row carries after those three cells is the auditor's
/// own arithmetic, which is recomputed rather than believed.
fn samples(text: &str) -> Vec<Sample> {
    let mut samples = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<&str>>();
        if cells.len() < 3 {
            continue;
        }
        let (Ok(regraded), Ok(agreed)) = (cells[1].parse::<usize>(), cells[2].parse::<usize>())
        else {
            continue;
        };
        let name = cells[0].trim_matches('*').trim().to_string();
        if name.is_empty() {
            continue;
        }
        samples.push(Sample {
            name,
            regraded,
            agreed,
        });
    }
    samples
}
