//! `weed rules` — the catalogue, as weed will apply it.
//!
//! Every rule, its default level, the face it belongs to, and the one line it
//! reports. The levels are read from `Config::default()`, so this face and the
//! judgement can never drift apart.

use crate::core::catalogue::{self, Face, Rule};
use crate::core::config::{Config, RuleSetting};
use crate::core::sarif::EXIT_CLEAN;
use crate::faces::Answer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Table,
    Json,
}

pub fn run(format: Format) -> Answer {
    let defaults = Config::default();
    let rows: Vec<Row> = catalogue::rules()
        .iter()
        .map(|rule| row(rule, &defaults))
        .collect();

    Answer {
        code: EXIT_CLEAN,
        stdout: match format {
            Format::Table => table(&rows),
            Format::Json => json(&rows),
        },
        stderr: Vec::new(),
    }
}

struct Row {
    id: String,
    level: String,
    face: String,
    finding: String,
    description: String,
}

fn row(rule: &Rule, defaults: &Config) -> Row {
    Row {
        id: rule.id.to_string(),
        level: level_word(defaults.rules.get(rule.id)).to_string(),
        face: face_word(rule.face).to_string(),
        finding: rule.short_description.to_string(),
        description: rule.full_description.to_string(),
    }
}

/// Check rules carry a level, scan rules are on or off. A rule the config never
/// heard of is off: weed will not run what it cannot configure.
fn level_word(setting: Option<&RuleSetting>) -> &'static str {
    match setting {
        Some(RuleSetting::Block) => "block",
        Some(RuleSetting::Warn) => "warn",
        Some(RuleSetting::On) => "on",
        Some(RuleSetting::Off) | None => "off",
    }
}

fn face_word(face: Face) -> &'static str {
    match face {
        Face::Check => "check",
        Face::Scan => "scan",
    }
}

fn table(rows: &[Row]) -> String {
    let id_width = width(rows.iter().map(|row| row.id.len()));
    let level_width = width(rows.iter().map(|row| row.level.len()));
    let face_width = width(rows.iter().map(|row| row.face.len()));

    let mut table = String::new();
    for row in rows {
        let line = format!(
            "{id:<id_width$}  {level:<level_width$}  {face:<face_width$}  {finding}",
            id = row.id,
            level = row.level,
            face = row.face,
            finding = row.finding,
        );
        table.push_str(line.trim_end());
        table.push('\n');
    }
    table
}

fn json(rows: &[Row]) -> String {
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "level": row.level,
                "face": row.face,
                "finding": row.finding,
                "description": row.description,
            })
        })
        .collect();
    // Every value here is a string, so there is nothing serde_json can refuse.
    format!(
        "{}\n",
        serde_json::to_string_pretty(&entries)
            .expect("a catalogue row holds only strings serde_json can write")
    )
}

fn width(lengths: impl Iterator<Item = usize>) -> usize {
    lengths.max().unwrap_or_default()
}
