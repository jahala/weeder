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
