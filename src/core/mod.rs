pub mod classify;
pub mod config;
pub mod diff;
pub mod finding;
pub mod suppress;

pub use classify::{classify_file, Classification, FileKind, Lang};
pub use config::{parse_config, Config, ConfigError, DependencyDirection, RuleSetting, Thresholds};
pub use diff::{parse_diff, ChangeKind, DiffError, FileDiff, Hunk, HunkLine, LineKind};
pub use finding::{Finding, Fix, Level, Message, Region};
pub use suppress::{
    apply_suppressions, parse_commit_suppressions, parse_inline_suppressions,
    InlineSuppressionError, Suppression, SuppressionSource,
};
