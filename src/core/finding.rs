#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Block,
    Warn,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub what: String,
    pub why: String,
    pub next: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub description: String,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,
    pub level: Level,
    pub path: String,
    pub region: Option<Region>,
    pub message: Message,
    pub fix: Option<Fix>,
    pub suppressed: Option<crate::core::suppress::Suppression>,
}

impl Level {
    /// The quieter of this level and `cap`: what a finding reports at when
    /// `cap` is the loudest its repository allows the rule.
    #[must_use]
    pub fn at_most(self, cap: Level) -> Level {
        if self.loudness() > cap.loudness() {
            cap
        } else {
            self
        }
    }

    fn loudness(self) -> u8 {
        match self {
            Level::Block => 2,
            Level::Warn => 1,
            Level::Note => 0,
        }
    }
}

/// The level a detector asks for one finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stamp {
    /// The rule's level: the catalogue default, or what `[rules]` sets.
    Rule,
    /// A level of the detector's own for this one finding, because it knows
    /// something the config does not. A level the repository wrote for the rule
    /// still caps it.
    Override(Level),
}

/// One finding as a detector reports it. The detector says what it found and
/// which level it asks for; the dispatch table names the rule and settles the
/// level, so a detector can get neither wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub stamp: Stamp,
    pub path: String,
    pub region: Option<Region>,
    pub message: Message,
    pub fix: Option<Fix>,
}

impl Detection {
    /// The finding this detection is reported as, under its rule's id and at
    /// the level settled for it.
    #[must_use]
    pub fn reported(self, rule: &str, level: Level) -> Finding {
        Finding {
            rule: rule.to_string(),
            level,
            path: self.path,
            region: self.region,
            message: self.message,
            fix: self.fix,
            suppressed: None,
        }
    }
}
