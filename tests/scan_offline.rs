//! A scan opens no network connection.
//!
//! R4 asks how far a pin is behind the registry, and the answer a judge gives
//! has to be the same answer in CI six months from now: what the registries
//! last said lives in a committed snapshot, and reading it is arithmetic.
//! `weeder scan --refresh-snapshot` is the one command that goes and asks.
//!
//! The proof is a real denial rather than a promise. Every run here is started
//! inside whatever this machine uses to take the network away from a process,
//! `sandbox-exec` on macOS and `unshare` on Linux, and the refresh under that
//! denial is what shows the denial bites: it comes back saying the registry
//! could not be reached. A denial that did nothing would let the refresh
//! through to crates.io and fail that test. The last two tests ask the source
//! tree and the catalogue the same question from the other side: nothing in
//! `src/` opens a socket, and `weeder rules` says which rule may reach a network
//! and under which flag.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{binary, command_in, fixture, weeder_in, Repo, Run};
use tempfile::TempDir;

/// The rule the registries belong to, and the one rule that may reach one.
const FETCHING_RULE: &str = "R4";

/// The flag that lets it, and the only way weeder leaves the machine.
const FETCHING_FLAG: &str = "--refresh-snapshot";

/// What a rule that reads what the repository already holds says it reaches.
const NOTHING: &str = "none";

/// How a Rust program opens a socket, and the crates that open one for it. weeder
/// reads a tree and writes a verdict; either of these in `src/` would be a judge
/// that had started making calls of its own.
const SOCKETS: &[&str] = &[
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "SocketAddr",
];

/// The crates a Rust program speaks HTTP or raw sockets through. None of them
/// is a dependency, and nothing in the source reaches for one. weeder names curl
/// as a program it asks, which is a string rather than a crate, so a source line
/// is read for the two shapes a crate is reached for by.
const CLIENTS: &[&str] = &[
    "reqwest",
    "ureq",
    "hyper",
    "isahc",
    "attohttpc",
    "surf",
    "curl",
    "socket2",
];

/// The denial is `sandbox-exec` or `unshare`, and Windows has neither: taking
/// the network away from a process there is a job object or a firewall rule, and
/// neither can be asked for from a test. The two proofs below hold the other
/// side on every platform: nothing in `src/` opens a socket, and the catalogue
/// says which rule may reach one.
#[cfg(unix)]
#[test]
fn a_scan_under_a_denied_network_leaves_clean_with_findings() {
    let repo = fixture(FETCHING_RULE, "rs", "fire");
    let run = denied(&repo, &["scan", "--format", "sarif"]);

    assert_eq!(
        run.code, 0,
        "a scan never blocks and never needs a network: {}{}",
        run.stdout, run.stderr
    );
    let found = run.findings();
    assert!(
        found.iter().any(|finding| finding.rule == FETCHING_RULE),
        "the pin is nine minor releases behind what the committed snapshot holds, and weeder with no network reported: {found:#?}"
    );
}

#[cfg(unix)]
#[test]
fn a_refresh_under_a_denied_network_says_the_registry_was_unreachable() {
    let repo = fixture(FETCHING_RULE, "rs", "fire");
    let snapshot = repo.root().join(".weeder/registry-snapshot.json");
    let before = std::fs::read_to_string(&snapshot).expect("the fixture commits a snapshot");

    let run = denied(&repo, &["scan", FETCHING_FLAG, "--format", "sarif"]);

    assert_eq!(
        run.code, 3,
        "a refresh that reached no registry did not happen, and weeder says so rather than judging on it: {}{}",
        run.stdout, run.stderr
    );
    let said = format!("{}{}", run.stdout, run.stderr);
    assert!(
        said.contains("crates.io") && said.contains("could not reach"),
        "the message names the registry weeder could not reach: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(&snapshot).expect("the snapshot should still be readable"),
        before,
        "a refresh that reached nothing left the committed snapshot as it found it, or the next scan would measure against an empty one"
    );
}

#[test]
fn nothing_in_the_source_opens_a_socket() {
    let mut found = Vec::new();
    for file in rust_files(&source_root()) {
        let source = std::fs::read_to_string(&file).expect("a source file should read");
        for (index, line) in source.lines().enumerate() {
            for spelling in SOCKETS {
                if line.contains(spelling) {
                    found.push(format!("{}:{}: {spelling}", file.display(), index + 1));
                }
            }
            for name in CLIENTS {
                if reaches_for(line, name) {
                    found.push(format!("{}:{}: {name}", file.display(), index + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "weeder reaches a registry by asking curl for it, so nothing in src/ speaks a network itself:\n{}",
        found.join("\n")
    );

    let clients: Vec<String> = dependencies()
        .into_iter()
        .filter(|name| CLIENTS.contains(&name.as_str()))
        .collect();
    assert!(
        clients.is_empty(),
        "a network client linked into the binary is a network weeder could reach without anybody asking: {clients:?}"
    );
}

#[test]
fn the_catalogue_names_the_one_rule_that_may_reach_a_network() {
    let anywhere = TempDir::new().expect("a directory to run in");
    let run = weeder_in(anywhere.path(), &["rules", "--format", "json"]);
    assert_eq!(run.code, 0);

    let entries: Vec<serde_json::Value> = serde_json::from_str(&run.stdout).expect("json is json");
    let mut reaching = Vec::new();
    for entry in &entries {
        let id = entry["id"].as_str().expect("a rule has an id");
        let network = entry["network"]
            .as_str()
            .expect("every rule says what it may reach over a network");
        if id == FETCHING_RULE {
            assert!(
                network.contains(FETCHING_FLAG),
                "{id} says which flag lets it reach a registry: {network}"
            );
            reaching.push(id.to_string());
        } else {
            assert_eq!(
                network, NOTHING,
                "{id} reads what the repository already holds, and says so"
            );
        }
    }
    assert_eq!(
        reaching,
        vec![FETCHING_RULE.to_string()],
        "one rule reaches a network, and it is the one whose question is what the registries released"
    );

    // The table carries the same column, between the face and what the rule
    // reports, so the answer is in front of whoever runs `weeder rules`.
    let table = weeder_in(anywhere.path(), &["rules"]);
    assert_eq!(table.code, 0);
    let lines = table.stdout_lines();
    assert_eq!(lines.len(), entries.len(), "one line per rule");
    for (line, entry) in lines.iter().zip(&entries) {
        // Two spaces or more part one column from the next; a cell's own words
        // are one space apart, so the row comes back whole.
        let cells: Vec<&str> = line
            .split("  ")
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();
        assert_eq!(
            cells,
            vec![
                entry["id"].as_str().unwrap_or_default(),
                entry["level"].as_str().unwrap_or_default(),
                entry["face"].as_str().unwrap_or_default(),
                entry["network"].as_str().unwrap_or_default(),
                entry["finding"].as_str().unwrap_or_default(),
            ],
            "the row reads id, level, face, network, finding: {line}"
        );
    }
}

/// The built binary, run in a repository with the network taken away from the
/// process before it starts. Everything else about the run is what any other
/// test does.
#[cfg(unix)]
fn denied(repo: &Repo, arguments: &[&str]) -> Run {
    let denial = Denial::on_this_machine();
    let weeder = binary().display().to_string();
    let mut wrapped: Vec<&str> = denial.arguments.iter().map(String::as_str).collect();
    wrapped.push(&weeder);
    wrapped.extend(arguments);

    let output = command_in(Path::new(&denial.program), repo.root(), &wrapped)
        .output()
        .expect("the denial should start the binary");
    Run {
        code: output
            .status
            .code()
            .expect("weeder should leave with a code"),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

/// How this machine takes the network away from a process it starts.
#[cfg(unix)]
struct Denial {
    program: String,
    arguments: Vec<String>,
}

#[cfg(unix)]
impl Denial {
    /// The first mechanism this machine can actually start a process with. A
    /// machine with none of them cannot hold this proof up, and a test that
    /// quietly passed on such a machine would be worth nothing: it fails and
    /// says which mechanisms it looked for.
    fn on_this_machine() -> Denial {
        let candidates = vec![
            // macOS: a profile that allows everything the binary does and
            // refuses every socket it might open.
            Denial {
                program: "sandbox-exec".to_string(),
                arguments: vec![
                    "-p".to_string(),
                    "(version 1)(allow default)(deny network*)".to_string(),
                ],
            },
            // Linux: a network namespace of its own, holding nothing but a
            // loopback interface that is never brought up.
            Denial {
                program: "unshare".to_string(),
                arguments: vec![
                    "--user".to_string(),
                    "--map-root-user".to_string(),
                    "--net".to_string(),
                ],
            },
        ];
        for candidate in candidates {
            if candidate.starts_a_process() {
                return candidate;
            }
        }
        panic!(
            "this machine has neither sandbox-exec nor unshare, so the network cannot be taken away from a process and none of this is proven"
        );
    }

    /// Whether the mechanism is here and runs what it is handed. What it does
    /// to the network is what the refresh test asks; a mechanism that let the
    /// network through would let that refresh reach crates.io and fail.
    fn starts_a_process(&self) -> bool {
        let mut command = Command::new(&self.program);
        command.args(&self.arguments).args(["/bin/echo", "started"]);
        command
            .output()
            .is_ok_and(|output| output.status.success() && output.stdout.starts_with(b"started"))
    }
}

/// Whether a line reaches for a crate: the path it writes a name from it with,
/// or the import that brings one in.
fn reaches_for(line: &str, crate_name: &str) -> bool {
    line.contains(&format!("{crate_name}::")) || line.contains(&format!("use {crate_name}"))
}

/// weeder's own source, the tree this test reads.
fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every Rust file under a directory, however deep.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(root).expect("the source tree should be readable");
    for entry in entries {
        let path = entry.expect("a source entry should be readable").path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// What the binary links against: the names in `[dependencies]`, and nothing
/// from the sections a test or a build reads.
fn dependencies() -> Vec<String> {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("the manifest should read");
    let mut names = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[dependencies]";
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            names.push(name.trim().to_string());
        }
    }
    names
}
