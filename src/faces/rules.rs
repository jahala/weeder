//! `weeder rules`, the catalogue, as weeder will apply it.
//!
//! Every rule, its default level, the face it belongs to, what it may reach
//! over the network, and the one line it reports, under a header naming each
//! column. The levels are read from `Config::default()`, so this face and the
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
    network: String,
    finding: String,
    description: String,
}

fn row(rule: &Rule, defaults: &Config) -> Row {
    Row {
        id: rule.id.to_string(),
        level: level_word(defaults.setting(rule)).to_string(),
        face: face_word(rule.face).to_string(),
        network: rule.network.spelled(),
        finding: rule.short_description.to_string(),
        description: rule.full_description.to_string(),
    }
}

/// Check rules carry a level, scan rules are on or off.
fn level_word(setting: RuleSetting) -> &'static str {
    match setting {
        RuleSetting::Block => "block",
        RuleSetting::Warn => "warn",
        RuleSetting::On => "on",
        RuleSetting::Off => "off",
    }
}

fn face_word(face: Face) -> &'static str {
    match face {
        Face::Check => "check",
        Face::Scan => "scan",
        Face::Bite => "bite",
    }
}

/// The names the table prints over its columns, the same names the json gives
/// its keys. A column nobody named gets read as whatever its reader guesses:
/// the network column's `none` was once taken for "this rule takes no
/// allowance" (#106).
const COLUMNS: [&str; 5] = ["id", "level", "face", "network", "finding"];

fn table(rows: &[Row]) -> String {
    let [id, level, face, network, finding] = COLUMNS;
    let id_width = width(rows.iter().map(|row| row.id.len()).chain([id.len()]));
    let level_width = width(rows.iter().map(|row| row.level.len()).chain([level.len()]));
    let face_width = width(rows.iter().map(|row| row.face.len()).chain([face.len()]));
    let network_width = width(
        rows.iter()
            .map(|row| row.network.len())
            .chain([network.len()]),
    );

    let line = |id: &str, level: &str, face: &str, network: &str, finding: &str| {
        let line = format!(
            "{id:<id_width$}  {level:<level_width$}  {face:<face_width$}  {network:<network_width$}  {finding}"
        );
        format!("{}\n", line.trim_end())
    };
    let mut table = line(id, level, face, network, finding);
    for row in rows {
        table.push_str(&line(
            &row.id,
            &row.level,
            &row.face,
            &row.network,
            &row.finding,
        ));
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
                "network": row.network,
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
