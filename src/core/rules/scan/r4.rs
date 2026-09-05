//! R4, a dependency pin lags the registry.
//!
//! A pin left far enough behind stops being a choice and becomes a liability:
//! the upgrade path gets longer every release, and the security fixes are on
//! the other side of it. What "far enough" means is the repository's call,
//! through `[thresholds] dependency_lag`.
//!
//! The comparison is against `.weed/registry-snapshot.json`, a file the
//! repository commits, and never against a registry. A scan that reached the
//! network would answer differently on every machine and would fail in CI on a
//! Monday morning for reasons that have nothing to do with the change under
//! review. `weed scan --refresh-snapshot` is the one command that goes and asks.

use crate::core::config::Config;
use crate::core::finding::{Finding, Level, Message, Region};
use crate::core::registry::{self, Behind, Pin, Registry, Version};
use crate::core::tree::Tree;

pub fn evaluate(tree: &Tree, config: &Config) -> Vec<Finding> {
    let allowed = config.thresholds.dependency_lag;
    let mut findings = Vec::new();
    for file in &tree.files {
        let Some(registry) = registry::registry_of(&file.path) else {
            continue;
        };
        for pin in registry::pins(registry, file.text()) {
            let Some(pinned) = pin.version else {
                continue;
            };
            let Some(latest) = tree.snapshot.latest(registry, &pin.package) else {
                continue;
            };
            let Some(behind) = registry::behind(pinned, latest) else {
                continue;
            };
            if !behind.beyond(allowed) {
                continue;
            }
            findings.push(finding(
                &file.path, registry, &pin, pinned, latest, behind, allowed,
            ));
        }
    }
    findings
}

fn finding(
    path: &str,
    registry: Registry,
    pin: &Pin,
    pinned: Version,
    latest: Version,
    behind: Behind,
    allowed: u32,
) -> Finding {
    Finding {
        rule: "R4".to_string(),
        level: Level::Warn,
        path: path.to_string(),
        region: (pin.line > 0).then_some(Region {
            start_line: pin.line,
            end_line: pin.line,
        }),
        message: Message {
            what: format!(
                "`{package}` is pinned at {pinned} and {registry} last carried {latest}, {distance} behind.",
                package = pin.package,
                registry = registry.name(),
                distance = behind.spelled(),
            ),
            why: format!(
                "the repository allows {allowed} minor releases of lag, and every release past that makes the upgrade longer and leaves the fixes on the far side of it."
            ),
            next: format!("upgrade `{}`, or raise [thresholds] dependency_lag and say why.", pin.package),
        },
        fix: None,
        suppressed: None,
    }
}
