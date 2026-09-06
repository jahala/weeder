//! The independent re-grade, and what the verdict may say because of it.
//!
//! Whether a block was a false positive is a judgement, and the builder's own
//! judgement is not the number weed ships on. A second party re-classifies a
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

use std::path::Path;

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

    fn stands(&self) -> bool {
        self.regraded >= SAMPLE_FLOOR
            && self.agreed <= self.regraded
            && self.agreement() >= AGREEMENT_BAR
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
}

/// The audit a run reads when no other path is given.
pub const DEFAULT_PATH: &str = "docs/calibration-audit-2026-09.md";

/// Read the re-grade. A file that is not there is not an error: the re-grade is
/// work that happens after the calibration, and its absence is exactly what the
/// provisional wording is for.
pub fn read(path: &Path, label: &str) -> Audit {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Audit {
            label: label.to_string(),
            samples: None,
        };
    };
    Audit {
        label: label.to_string(),
        samples: Some(samples(&text)),
    }
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
