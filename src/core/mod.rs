pub mod catalogue;
pub mod change;
pub mod classify;
pub mod config;
pub mod diff;
pub mod finding;
pub mod glob;
pub mod guard;
pub mod read;
pub mod rules;
pub mod sarif;
pub mod suppress;
pub mod syntax;

pub use catalogue::{Face, Rule};
pub use change::{Change, Side};
pub use classify::{classify_file, Classification, FileKind, Lang};
pub use config::{parse_config, Config, ConfigError, DependencyDirection, RuleSetting, Thresholds};
pub use diff::{parse_diff, ChangeKind, DiffError, FileDiff, Hunk, HunkLine, LineKind};
pub use finding::{Finding, Fix, Level, Message, Region};
pub use guard::{parse_push_refs, protects, Hook, PushRef};
pub use read::{
    CallerSite, Definition, DefinitionKind, Import, Outline, TestShape, TestUnit, TestUnitKind,
};
pub use sarif::{Context, Log, Outcome};
pub use suppress::{
    apply_suppressions, parse_commit_suppressions, parse_inline_suppressions,
    InlineSuppressionError, Suppression, SuppressionSource,
};
pub use syntax::{Mask, Syntax, Word};
