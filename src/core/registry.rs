//! Dependency pins, and how far behind the registry each one is.
//!
//! Reading a manifest is parsing, and comparing two versions is arithmetic;
//! neither needs the network, and a scan never reaches it. What the registries
//! last said lives in a committed snapshot, so the answer weeder gives on a
//! developer's machine is the answer it gives in CI six months later.
//! `weeder scan --refresh-snapshot` is the one command that goes and asks, and the
//! knowledge of where to ask and what to read out of the reply lives here so the
//! seam that fetches stays a fetch.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Where the snapshot lives, relative to the repository root.
pub const SNAPSHOT_PATH: &str = ".weeder/registry-snapshot.json";

/// A registry weeder knows how to read a manifest for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Registry {
    Npm,
    Cargo,
    Pypi,
    Go,
}

impl Registry {
    /// The word the snapshot files this registry's packages under.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Registry::Npm => "npm",
            Registry::Cargo => "cargo",
            Registry::Pypi => "pypi",
            Registry::Go => "go",
        }
    }

    /// What a person calls it, for a finding's message.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Registry::Npm => "the npm registry",
            Registry::Cargo => "crates.io",
            Registry::Pypi => "PyPI",
            Registry::Go => "the Go module proxy",
        }
    }

    /// Where the registry answers "what is the latest release of this package".
    #[must_use]
    pub fn latest_url(self, package: &str) -> String {
        match self {
            Registry::Npm => format!("https://registry.npmjs.org/{package}"),
            Registry::Cargo => format!("https://crates.io/api/v1/crates/{package}"),
            Registry::Pypi => format!("https://pypi.org/pypi/{package}/json"),
            Registry::Go => format!(
                "https://proxy.golang.org/{}/@latest",
                escape_module(package)
            ),
        }
    }

    /// The version in a registry's reply, or `None` where the reply is not one
    /// this registry writes.
    #[must_use]
    pub fn latest_in(self, reply: &str) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(reply).ok()?;
        let found = match self {
            Registry::Npm => value.get("dist-tags")?.get("latest")?,
            Registry::Cargo => value.get("crate")?.get("max_stable_version")?,
            Registry::Pypi => value.get("info")?.get("version")?,
            Registry::Go => value.get("Version")?,
        };
        found.as_str().map(ToString::to_string)
    }
}

/// The Go module proxy spells an uppercase letter in a module path as `!` and
/// the lowercase letter, so two paths that differ only in case stay two paths
/// on a case-insensitive filesystem.
fn escape_module(path: &str) -> String {
    path.chars()
        .flat_map(|character| {
            if character.is_ascii_uppercase() {
                vec!['!', character.to_ascii_lowercase()]
            } else {
                vec![character]
            }
        })
        .collect()
}

/// Which registry a manifest's dependencies come from, or `None` where the file
/// is not a manifest weeder reads.
#[must_use]
pub fn registry_of(path: &str) -> Option<Registry> {
    match path.rsplit('/').next().unwrap_or(path) {
        "package.json" => Some(Registry::Npm),
        "Cargo.toml" => Some(Registry::Cargo),
        "pyproject.toml" => Some(Registry::Pypi),
        "go.mod" => Some(Registry::Go),
        _ => None,
    }
}

/// A release, as far as a lag is concerned. A pre-release tail is kept so a
/// message can say what the manifest actually holds, and is not compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

/// The version a requirement pins to, or `None` where it names none: a git
/// dependency, a workspace member, a `*`.
#[must_use]
pub fn parse_version(requirement: &str) -> Option<Version> {
    let digits = requirement
        .trim()
        .trim_start_matches(|character: char| !character.is_ascii_digit());
    let mut numbers = digits
        .split(|character: char| !character.is_ascii_digit())
        .map(str::parse::<u64>);
    let major = numbers.next()?.ok()?;
    let minor = numbers.next().and_then(Result::ok).unwrap_or(0);
    let patch = numbers.next().and_then(Result::ok).unwrap_or(0);
    Some(Version {
        major,
        minor,
        patch,
    })
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// How far one version is behind another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Behind {
    pub majors: u64,
    /// Minor releases behind, where the two share a major. A pin behind by a
    /// major is behind by a whole series, and the minors of two different
    /// series do not subtract.
    pub minors: u64,
}

impl Behind {
    /// Whether this is further behind than a repository allows. A major behind
    /// is beyond any minor allowance: the series moved on.
    #[must_use]
    pub fn beyond(&self, allowed: u32) -> bool {
        self.majors > 0 || self.minors > u64::from(allowed)
    }

    /// The distance in words, for a finding's message.
    #[must_use]
    pub fn spelled(&self) -> String {
        if self.majors > 0 {
            plural(self.majors, "major release")
        } else {
            plural(self.minors, "minor release")
        }
    }
}

fn plural(count: u64, word: &str) -> String {
    if count == 1 {
        format!("{count} {word}")
    } else {
        format!("{count} {word}s")
    }
}

/// How far `pinned` is behind `latest`, or `None` where it is not behind at all.
#[must_use]
pub fn behind(pinned: Version, latest: Version) -> Option<Behind> {
    if latest.major > pinned.major {
        return Some(Behind {
            majors: latest.major - pinned.major,
            minors: 0,
        });
    }
    if latest.major == pinned.major && latest.minor > pinned.minor {
        return Some(Behind {
            majors: 0,
            minors: latest.minor - pinned.minor,
        });
    }
    None
}

/// One dependency a manifest pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub package: String,
    /// What the manifest wrote: `^1.2.0`, `>=2,<3`, `v1.9.0`.
    pub requirement: String,
    /// The release the requirement names, or `None` where it names none.
    pub version: Option<Version>,
    /// 1-based, so a finding points at the line that holds the pin.
    pub line: u32,
}

/// Every dependency a manifest pins, in the order the file writes them. A
/// manifest weeder cannot parse pins nothing: a scan reports what a repository
/// holds, and never reports its own inability to read as the repository's fault.
#[must_use]
pub fn pins(registry: Registry, content: &str) -> Vec<Pin> {
    let named = match registry {
        Registry::Npm => npm_pins(content),
        Registry::Cargo => cargo_pins(content),
        Registry::Pypi => pypi_pins(content),
        Registry::Go => return go_pins(content),
    };
    named
        .into_iter()
        .map(|(package, requirement)| Pin {
            line: line_of(content, &package),
            version: parse_version(&requirement),
            package,
            requirement,
        })
        .collect()
}

/// The 1-based line a package is named on, or 0 where the name is nowhere in
/// the text, a finding with no line points at the file.
fn line_of(content: &str, package: &str) -> u32 {
    content
        .lines()
        .position(|line| line.contains(package))
        .map_or(0, |index| index as u32 + 1)
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    dependencies: Option<BTreeMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<BTreeMap<String, String>>,
    #[serde(rename = "peerDependencies")]
    peer_dependencies: Option<BTreeMap<String, String>>,
}

fn npm_pins(content: &str) -> Vec<(String, String)> {
    let Ok(manifest) = serde_json::from_str::<PackageJson>(content) else {
        return Vec::new();
    };
    [
        manifest.dependencies,
        manifest.dev_dependencies,
        manifest.peer_dependencies,
    ]
    .into_iter()
    .flatten()
    .flatten()
    .collect()
}

fn cargo_pins(content: &str) -> Vec<(String, String)> {
    let Ok(manifest) = toml::from_str::<toml::Value>(content) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(entries) = manifest.get(table).and_then(toml::Value::as_table) else {
            continue;
        };
        for (package, value) in entries {
            // A dependency is either the version itself or a table that may
            // carry one; a git or a path dependency carries no version, and a
            // registry has nothing to say about it.
            let requirement = match value {
                toml::Value::String(version) => Some(version.clone()),
                toml::Value::Table(table) => table
                    .get("version")
                    .and_then(toml::Value::as_str)
                    .map(ToString::to_string),
                _ => None,
            };
            if let Some(requirement) = requirement {
                found.push((package.clone(), requirement));
            }
        }
    }
    found
}

fn pypi_pins(content: &str) -> Vec<(String, String)> {
    let Ok(manifest) = toml::from_str::<toml::Value>(content) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    // PEP 621 writes a list of requirement strings.
    if let Some(listed) = manifest
        .get("project")
        .and_then(|project| project.get("dependencies"))
        .and_then(toml::Value::as_array)
    {
        for entry in listed.iter().filter_map(toml::Value::as_str) {
            if let Some(pin) = requirement_string(entry) {
                found.push(pin);
            }
        }
    }
    // Poetry writes a table of package to requirement.
    if let Some(entries) = manifest
        .get("tool")
        .and_then(|tool| tool.get("poetry"))
        .and_then(|poetry| poetry.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (package, value) in entries {
            let requirement = match value {
                toml::Value::String(version) => Some(version.clone()),
                toml::Value::Table(table) => table
                    .get("version")
                    .and_then(toml::Value::as_str)
                    .map(ToString::to_string),
                _ => None,
            };
            if let Some(requirement) = requirement {
                found.push((package.clone(), requirement));
            }
        }
    }
    found
}

/// A PEP 508 requirement split into the package and what it demands:
/// `requests>=2.31.0`, `django[argon2] ~= 4.2`.
fn requirement_string(entry: &str) -> Option<(String, String)> {
    let entry = entry.trim();
    let end = entry
        .find(|character: char| !(character.is_alphanumeric() || "-_.".contains(character)))
        .unwrap_or(entry.len());
    let package = entry[..end].trim();
    if package.is_empty() {
        return None;
    }
    Some((package.to_string(), entry[end..].trim().to_string()))
}

/// `go.mod` is a line format: a `require` on its own, or a parenthesised block
/// of them. An indirect requirement is another module's choice rather than this
/// repository's, so it is not this repository's pin to answer for.
fn go_pins(content: &str) -> Vec<Pin> {
    let mut found = Vec::new();
    let mut inside_block = false;
    for (index, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with("//") || line.contains("// indirect") {
            continue;
        }
        let entry = if inside_block {
            if line == ")" {
                inside_block = false;
                continue;
            }
            line
        } else if let Some(rest) = line.strip_prefix("require ") {
            let rest = rest.trim();
            if rest == "(" {
                inside_block = true;
                continue;
            }
            rest
        } else {
            if line == "require (" {
                inside_block = true;
            }
            continue;
        };

        let mut words = entry.split_whitespace();
        let (Some(package), Some(requirement)) = (words.next(), words.next()) else {
            continue;
        };
        found.push(Pin {
            package: package.to_string(),
            requirement: requirement.to_string(),
            version: parse_version(requirement),
            line: index as u32 + 1,
        });
    }
    found
}

/// What the registries last said, as the repository committed it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    /// Registry key, then package, then the latest release.
    entries: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotError {
    pub message: String,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{SNAPSHOT_PATH} is not valid: {}. write it again with weeder scan --refresh-snapshot, or delete it.",
            self.message
        )
    }
}

impl std::error::Error for SnapshotError {}

impl Snapshot {
    /// The latest release the snapshot recorded, or `None` where it recorded
    /// none for that package.
    #[must_use]
    pub fn latest(&self, registry: Registry, package: &str) -> Option<Version> {
        self.recorded(registry, package).and_then(parse_version)
    }

    /// The release the snapshot recorded, as the registry spelled it. A refresh
    /// that could not reach a registry carries the old answer across rather than
    /// dropping the package: a snapshot with nothing in it for a package is a
    /// rule with nothing to say about it.
    #[must_use]
    pub fn recorded(&self, registry: Registry, package: &str) -> Option<&str> {
        self.entries
            .get(registry.key())
            .and_then(|packages| packages.get(package))
            .map(String::as_str)
    }

    pub fn record(&mut self, registry: Registry, package: &str, version: &str) {
        self.entries
            .entry(registry.key().to_string())
            .or_default()
            .insert(package.to_string(), version.to_string());
    }

    /// The snapshot as the file the repository commits. Two maps sorted by key
    /// write the same bytes on every machine.
    #[must_use]
    pub fn to_json(&self) -> String {
        // Every value is a string keyed by a string, so there is nothing here
        // serde_json can refuse to write.
        format!(
            "{}\n",
            serde_json::to_string_pretty(&self.entries)
                .expect("a snapshot holds only strings serde_json can write")
        )
    }
}

/// The snapshot a repository committed. A repository with no snapshot has an
/// empty one, which is a rule with nothing to compare against rather than a
/// failure.
pub fn parse_snapshot(text: Option<&str>) -> Result<Snapshot, SnapshotError> {
    let Some(text) = text else {
        return Ok(Snapshot::default());
    };
    if text.trim().is_empty() {
        return Ok(Snapshot::default());
    }
    let entries = serde_json::from_str(text).map_err(|error| SnapshotError {
        message: error.to_string(),
    })?;
    Ok(Snapshot { entries })
}
