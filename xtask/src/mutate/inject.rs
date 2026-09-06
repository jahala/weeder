//! One anti-pattern, planted once, in a real commit's tree.
//!
//! Every injection here is the shape the rule exists for, written the way the
//! language writes it and nothing like the way weed detects it: a case taken
//! out of a suite, a skip marker on a case, a body that does nothing, a handler
//! that catches and says nothing, a credential assigned to a name that says so,
//! a conflict left in the file, a guardrail edited. The site is found with the
//! injector's own scanner,
//! so a rule that cannot see a shape is given one to miss rather than spared it.
//!
//! One mutation per case, always. A hit has to be attributable to one rule and a
//! miss to one site, which is the whole reason the campaign is worth running.

use super::source::{basename, directory, extension, Case, Language, Source};
use super::tree::Tree;

/// One planted anti-pattern: what was written, where the finding is expected,
/// and what the run needs beyond its base arguments.
#[derive(Debug, Clone)]
pub struct Mutation {
    /// What was planted, one line, for the report and for the miss list.
    pub shape: String,
    /// The paths a finding may land on for this to count as caught.
    pub target: Vec<String>,
    /// The lines the finding is expected on, one-based and inclusive, where the
    /// shape is written on lines at all. A shape that is about a whole file, a
    /// deletion, a rename, a manifest, expects no particular line.
    pub lines: Option<(u32, u32)>,
    /// The files to write: the new text, or nothing to take the file away.
    pub writes: Vec<(String, Option<String>)>,
    /// Arguments this case's run needs on top of `check --base <parent>`.
    pub arguments: Vec<String>,
    /// A `weed.toml` this case's run reads, written outside the repository so
    /// the config is not itself part of the diff being judged.
    pub config: Option<String>,
}

impl Mutation {
    fn new(shape: impl Into<String>, target: &str) -> Mutation {
        Mutation {
            shape: shape.into(),
            target: vec![target.to_string()],
            lines: None,
            writes: Vec::new(),
            arguments: Vec::new(),
            config: None,
        }
    }

    fn writing(mut self, path: &str, text: String) -> Mutation {
        self.writes.push((path.to_string(), Some(text)));
        self
    }

    fn removing(mut self, path: &str) -> Mutation {
        self.writes.push((path.to_string(), None));
        self
    }

    fn at(mut self, first: u32, last: u32) -> Mutation {
        self.lines = Some((first, last));
        self
    }

    fn also(mut self, path: &str) -> Mutation {
        self.target.push(path.to_string());
        self
    }
}

/// Why a commit gave a rule no case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSite {
    /// The tree holds nothing this shape can be planted in.
    Absent,
    /// The language's runner cannot be fooled this way at all: renaming a case
    /// that is collected by an attribute changes nothing about what runs.
    NotInLanguage,
}

/// The rules the campaign injects for, in catalogue order.
pub const RULES: &[&str] = &[
    "T1", "T2", "T3", "T4", "T5", "T6", "T7", "M1", "S1", "S2", "S3", "D1", "D2", "X1", "X2", "C1",
    "C2", "C3", "G1", "G2",
];

/// Plant one anti-pattern of this rule in this tree, or say why there is
/// nowhere to plant it.
pub fn inject(rule: &str, lang: Language, tree: &Tree, seed: u64) -> Result<Mutation, NoSite> {
    let planted = match rule {
        "T1" => delete_a_case(lang, tree, seed),
        "T2" => drop_an_assertion(lang, tree, seed),
        "T3" => add_a_skip(lang, tree, seed),
        "T4" => widen_a_slack(lang, tree, seed),
        "T5" => regenerate_an_expectation(lang, tree, seed),
        "T6" => weaken_an_error_assertion(lang, tree, seed),
        "T7" => return rename_out_of_the_runner(lang, tree, seed),
        "M1" => mock_the_unit_under_change(lang, tree, seed),
        "S1" => stub_production_code(lang, tree, seed),
        "S2" => swallow_a_failure(lang, tree, seed),
        "S3" => leave_a_print(lang, tree, seed),
        "D1" => change_a_manifest(lang, tree, seed),
        "D2" => cross_a_boundary(lang, tree, seed),
        "X1" => add_a_secret(lang, tree, seed),
        "X2" => touch_outside_the_scope(lang, tree, seed),
        "C1" => edit_a_guardrail(lang, tree, seed),
        "C2" => broaden_an_ignore(lang, tree, seed),
        "C3" => edit_a_workflow(lang, tree, seed),
        "G1" => commit_a_conflict(lang, tree, seed),
        "G2" => add_a_blob(lang, tree, seed),
        _ => None,
    };
    planted.ok_or(NoSite::Absent)
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// T1: a suite loses one of its cases.
fn delete_a_case(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.tests(lang), seed) {
        // A rule that counts what a file holds is only attributable in a file
        // the commit itself left alone: a suite the change added cases to can
        // lose one and still hold more than it did.
        if tree.touched(&source.path) {
            continue;
        }
        let cases = source.cases();
        if cases.len() < 2 {
            continue;
        }
        let case = &cases[(seed as usize) % cases.len()];
        let text = without(source, case.first as usize, case.last as usize);
        return Some(
            Mutation::new(
                format!("the case `{}` was deleted", case.name),
                &source.path,
            )
            .writing(&source.path, text),
        );
    }
    None
}

/// T2: a case keeps its name and stops claiming anything.
fn drop_an_assertion(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.tests(lang), seed) {
        if tree.touched(&source.path) {
            continue;
        }
        let claims = source.assertions();
        if claims.len() < 2 {
            continue;
        }
        let at = claims[(seed as usize) % claims.len()] as usize;
        let text = without(source, at, at);
        return Some(
            Mutation::new(
                format!("the assertion on line {} was deleted", at + 1),
                &source.path,
            )
            .writing(&source.path, text),
        );
    }
    None
}

/// T3: a case is turned off where it stands.
fn add_a_skip(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.tests(lang), seed) {
        let cases = source.cases();
        if cases.is_empty() {
            continue;
        }
        for offset in 0..cases.len() {
            let case = &cases[(seed as usize + offset) % cases.len()];
            let first = case.first as usize;
            let Some(planted) = skip_marker(source, case) else {
                continue;
            };
            return Some(match planted {
                Planted::Rewrite(line, written) => {
                    let mut lines = source.lines.clone();
                    *lines.get_mut(first)? = line;
                    Mutation::new(format!("`{written}` was put on a case"), &source.path)
                        .writing(&source.path, joined(&lines))
                        .at(first as u32 + 1, first as u32 + 1)
                }
                Planted::Insert(at, line, written) => {
                    let text = with(source, at, &[line]);
                    Mutation::new(format!("`{written}` was put on a case"), &source.path)
                        .writing(&source.path, text)
                        .at(at as u32 + 1, at as u32 + 1)
                }
            });
        }
    }
    None
}

enum Planted {
    /// The line at this index, rewritten.
    Rewrite(String, String),
    /// A line put in front of this index.
    Insert(usize, String, String),
}

fn skip_marker(source: &Source, case: &Case) -> Option<Planted> {
    let first = case.first as usize;
    let raw = source.lines.get(first)?;
    let code = source.code.get(first)?;
    match source.lang {
        Language::Ts => {
            let declarator = code.trim_start().split(['(', '.']).next()?.to_string();
            if code.contains(&format!("{declarator}.")) {
                // Already carries a member: another marker, or a table of cases.
                return None;
            }
            let at = raw.find(&format!("{declarator}("))?;
            let mut line = raw.clone();
            line.insert_str(at + declarator.len(), ".skip");
            Some(Planted::Rewrite(line, format!("{declarator}.skip")))
        }
        Language::Py => {
            let indent = leading(raw);
            let line = format!("{indent}@pytest.mark.skip(reason=\"unstable\")");
            Some(Planted::Insert(
                first,
                line,
                "@pytest.mark.skip".to_string(),
            ))
        }
        Language::Rs => {
            let indent = leading(raw);
            Some(Planted::Insert(
                first + 1,
                format!("{indent}#[ignore]"),
                "#[ignore]".to_string(),
            ))
        }
        Language::Go => {
            let handle = code
                .split_once('(')
                .and_then(|(_, arguments)| arguments.split_whitespace().next())
                .unwrap_or("t")
                .trim_end_matches(',')
                .to_string();
            if handle.is_empty() || !handle.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return None;
            }
            let indent = source.body_indent(first);
            Some(Planted::Insert(
                first + 1,
                format!("{indent}{handle}.Skip(\"unstable\")"),
                format!("{handle}.Skip"),
            ))
        }
    }
}

/// The words a number is a slack under: every one of them accepts more as it
/// grows, which is what makes multiplying it a widening rather than a change.
/// A name reads as one where it begins with the word, `waitFor`, `timeoutMs`,
/// so `await` is not a wait and a field called `attempts` in a payload is not a
/// retry count.
const SLACK_WORDS: &[&str] = &[
    "timeout",
    "delay",
    "sleep",
    "wait",
    "retries",
    "retry",
    "tolerance",
    "deadline",
    "backoff",
    "interval",
];

/// Where the first name that reads as a slack ends on this line, if one does.
fn slack_word(code: &str) -> Option<usize> {
    let characters: Vec<char> = code.chars().collect();
    let mut at = 0;
    while at < characters.len() {
        if !characters[at].is_alphabetic() && characters[at] != '_' {
            at += 1;
            continue;
        }
        let mut end = at;
        while end < characters.len()
            && (characters[end].is_alphanumeric() || characters[end] == '_')
        {
            end += 1;
        }
        let word: String = characters[at..end]
            .iter()
            .flat_map(|character| character.to_lowercase())
            .collect();
        if SLACK_WORDS.iter().any(|slack| word.starts_with(slack)) {
            return Some(end);
        }
        at = end;
    }
    None
}

/// T4: a wait or a tolerance grows until nothing fails.
///
/// The number that grows is the one written under the slack word, found in the
/// code rather than in the line: a year inside a string is not a timeout, and a
/// version is not a tolerance.
fn widen_a_slack(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.tests(lang), seed) {
        for (at, code) in source.code.iter().enumerate() {
            let Some(word) = slack_word(code) else {
                continue;
            };
            let raw = source.lines.get(at)?;
            let Some((first, last)) = number_span(code, word) else {
                continue;
            };
            let Some((widened, before, after)) = grown(raw, first, last) else {
                continue;
            };
            let mut lines = source.lines.clone();
            *lines.get_mut(at)? = widened;
            return Some(
                Mutation::new(
                    format!("a wait was widened from {before} to {after}"),
                    &source.path,
                )
                .writing(&source.path, joined(&lines))
                .at(at as u32 + 1, at as u32 + 1),
            );
        }
    }
    None
}

/// Where the first number after this position is written, as byte offsets. A
/// number touching a letter is part of a name, and one with two dots in it is a
/// version rather than a quantity.
fn number_span(code: &str, from: usize) -> Option<(usize, usize)> {
    let bytes: Vec<char> = code.chars().collect();
    let mut at = from.min(bytes.len());
    while at < bytes.len() {
        if !bytes[at].is_ascii_digit() {
            at += 1;
            continue;
        }
        if at > 0
            && (bytes[at - 1].is_alphanumeric() || bytes[at - 1] == '_' || bytes[at - 1] == '.')
        {
            at += 1;
            continue;
        }
        let mut end = at;
        let mut dotted = false;
        while end < bytes.len()
            && (bytes[end].is_ascii_digit()
                || (bytes[end] == '.'
                    && !dotted
                    && bytes.get(end + 1).is_some_and(char::is_ascii_digit)))
        {
            dotted |= bytes[end] == '.';
            end += 1;
        }
        if bytes
            .get(end)
            .is_some_and(|character| character.is_alphanumeric() || *character == '.')
        {
            at = end + 1;
            continue;
        }
        return Some((at, end));
    }
    None
}

/// The line with the number in that span multiplied by ten, and the number as
/// it was written and as it now reads.
fn grown(line: &str, first: usize, last: usize) -> Option<(String, String, String)> {
    let characters: Vec<char> = line.chars().collect();
    if last > characters.len() || first >= last {
        return None;
    }
    let written: String = characters[first..last].iter().collect();
    let value = written.parse::<f64>().ok()?;
    if value <= 0.0 {
        return None;
    }
    let decimals = written.split_once('.').map_or(0, |(_, tail)| tail.len());
    let grown = if decimals > 0 {
        format!("{:.*}", decimals, value * 10.0)
    } else {
        format!("{}", (value * 10.0) as i64)
    };
    let mut rewritten: String = characters[..first].iter().collect();
    rewritten.push_str(&grown);
    rewritten.extend(characters[last..].iter());
    Some((rewritten, written, grown))
}

/// The directories and extensions a runner keeps its recorded expectations in.
const EXPECTATION_DIRECTORIES: &[&str] = &[
    "__snapshots__",
    "snapshots",
    "testdata",
    "golden",
    "fixtures",
    "__fixtures__",
];
const EXPECTATION_EXTENSIONS: &[&str] = &["snap", "golden", "approved"];

/// T5: the expectation moves to meet the code the same change rewrote.
fn regenerate_an_expectation(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let touched_production = tree
        .production(lang)
        .into_iter()
        .any(|source| tree.touched(&source.path));
    if !touched_production {
        return None;
    }
    let recorded: Vec<&String> = tree
        .paths
        .iter()
        .filter(|path| is_expectation(path))
        .filter(|path| !tree.touched(path))
        .collect();
    for path in rotate(recorded, seed) {
        let Some(text) = tree.text(path) else {
            continue;
        };
        let Some(last) = text.lines().rev().find(|line| !line.trim().is_empty()) else {
            continue;
        };
        let mut written = text.clone();
        if !written.ends_with('\n') {
            written.push('\n');
        }
        written.push_str(last);
        written.push('\n');
        return Some(
            Mutation::new("a recorded expectation was rewritten", path).writing(path, written),
        );
    }
    None
}

fn is_expectation(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let in_directory = segments
        .iter()
        .any(|segment| EXPECTATION_DIRECTORIES.contains(segment));
    let recorded = extension(path).is_some_and(|found| EXPECTATION_EXTENSIONS.contains(&found));
    let approved = basename(path).contains(".approved.");
    (in_directory || recorded || approved) && !path.ends_with(".go") && !path.ends_with(".rs")
}

/// T6: the claim about a failure stops naming which failure.
fn weaken_an_error_assertion(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.tests(lang), seed) {
        for (at, code) in source.code.iter().enumerate() {
            let raw = source.lines.get(at)?;
            let Some(weakened) = weaker(lang, code, raw) else {
                continue;
            };
            let mut lines = source.lines.clone();
            *lines.get_mut(at)? = weakened;
            return Some(
                Mutation::new("an error assertion stopped naming the error", &source.path)
                    .writing(&source.path, joined(&lines))
                    .at(at as u32 + 1, at as u32 + 1),
            );
        }
    }
    None
}

fn weaker(lang: Language, code: &str, raw: &str) -> Option<String> {
    match lang {
        Language::Ts => {
            let call = ["toThrowError", "toThrow", "rejects.toThrow"]
                .into_iter()
                .find(|call| code.contains(&format!(".{call}(")))?;
            let at = raw.find(&format!(".{call}("))? + call.len() + 2;
            let end = closing(raw, at - 1)?;
            if raw[at..end].trim().is_empty() {
                return None;
            }
            Some(format!("{}{}", &raw[..at], &raw[end..]))
        }
        Language::Py => {
            let at = raw.find("raises(")? + "raises(".len();
            let end = closing(raw, at - 1)?;
            let named = raw[at..end].trim();
            if named.is_empty() || named == "Exception" {
                return None;
            }
            Some(format!("{}Exception{}", &raw[..at], &raw[end..]))
        }
        Language::Rs => {
            if !code.contains("Err(") {
                return None;
            }
            // Both shapes name the failure and both come apart the same way:
            // whatever is claimed about, followed by the kind it is claimed to
            // be. What is left is the claim with the kind taken out of it.
            let opened = ["matches!(", "assert_eq!(", "assert_ne!("]
                .into_iter()
                .find_map(|macro_name| raw.find(macro_name).map(|at| at + macro_name.len()))?;
            let subject = raw[opened..].split(',').next()?.trim().to_string();
            if subject.is_empty() || subject.contains(' ') || subject.contains("Err(") {
                return None;
            }
            Some(format!("{}assert!({subject}.is_err());", leading(raw)))
        }
        Language::Go => {
            let trimmed = code.trim();
            if !trimmed.starts_with("if ") || !trimmed.ends_with('{') {
                return None;
            }
            if !code.contains("errors.Is(") && !code.contains("errors.As(") {
                return None;
            }
            Some(format!("{}if err == nil {{", leading(raw)))
        }
    }
}

/// T7: the file keeps its tests and the runner stops collecting it.
fn rename_out_of_the_runner(lang: Language, tree: &Tree, seed: u64) -> Result<Mutation, NoSite> {
    if lang == Language::Rs && tree.paths.iter().all(|path| !collected_rust_target(path)) {
        // cargo collects a case by its attribute, so no rename of a case takes
        // one out of the run. Only an integration target has a name to lose,
        // and this tree carries none.
        return Err(NoSite::NotInLanguage);
    }
    for source in rotate(tree.tests(lang), seed) {
        if tree.touched(&source.path) || !lang.collects_file(&source.path) {
            continue;
        }
        let Some(renamed) = uncollected_name(lang, &source.path) else {
            continue;
        };
        if tree.paths.contains(&renamed) {
            continue;
        }
        return Ok(
            Mutation::new(format!("the suite was renamed to `{renamed}`"), &renamed)
                .also(&source.path)
                .removing(&source.path)
                .writing(&renamed, source.text()),
        );
    }
    Err(NoSite::Absent)
}

fn collected_rust_target(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    segments.len() == 2 && matches!(segments[0], "tests" | "benches") && path.ends_with(".rs")
}

/// The same file under a name its runner does not collect.
fn uncollected_name(lang: Language, path: &str) -> Option<String> {
    let name = basename(path);
    let directory = directory(path);
    let renamed = match lang {
        Language::Ts => {
            if directory.split('/').any(|segment| segment == "__tests__") {
                return None;
            }
            if name.contains(".test.") {
                name.replacen(".test.", ".checks.", 1)
            } else if name.contains(".spec.") {
                name.replacen(".spec.", ".checks.", 1)
            } else {
                return None;
            }
        }
        Language::Py => match (name.strip_prefix("test_"), name.strip_suffix("_test.py")) {
            (Some(rest), _) => format!("checks_{rest}"),
            (None, Some(stem)) => format!("{stem}_checks.py"),
            (None, None) => return None,
        },
        Language::Go => {
            let stem = name.strip_suffix("_test.go")?;
            format!("{stem}_checks.go")
        }
        // An integration target one directory down is a module nobody compiles.
        Language::Rs => {
            if !collected_rust_target(path) {
                return None;
            }
            return Some(format!("{directory}/support/{name}"));
        }
    };
    Some(if directory.is_empty() {
        renamed
    } else {
        format!("{directory}/{renamed}")
    })
}

/// M1: the suite doubles the very thing the change touched.
fn mock_the_unit_under_change(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let changed: Vec<&Source> = tree
        .production(lang)
        .into_iter()
        .filter(|source| tree.touched(&source.path))
        .collect();
    let under_change = rotate(changed, seed).next()?;
    for source in rotate(tree.tests(lang), seed) {
        if source.path == under_change.path {
            continue;
        }
        let at = source.last_import().map_or(0, |line| line + 1);
        let Some(double) = double_of(lang, source, under_change) else {
            continue;
        };
        let text = with(source, at, std::slice::from_ref(&double));
        return Some(
            Mutation::new(
                format!("`{}` was doubled in a suite", basename(&under_change.path)),
                &source.path,
            )
            .writing(&source.path, text)
            .at(at as u32 + 1, at as u32 + 1),
        );
    }
    None
}

fn double_of(lang: Language, test: &Source, production: &Source) -> Option<String> {
    match lang {
        Language::Ts => Some(format!(
            "jest.mock('{}');",
            relative_module(&test.path, &production.path)
        )),
        Language::Py => Some(format!("patch(\"{}\")", dotted_module(&production.path))),
        Language::Rs => {
            let name = exported_type(production)?;
            Some(format!("struct Mock{name};"))
        }
        Language::Go => {
            let name = exported_type(production)?;
            Some(format!("type mock{name} struct{{}}"))
        }
    }
}

/// A type the file makes available to the rest of the program.
fn exported_type(source: &Source) -> Option<String> {
    for code in &source.code {
        let trimmed = code.trim_start();
        let declared = match source.lang {
            Language::Rs => trimmed
                .strip_prefix("pub struct ")
                .or_else(|| trimmed.strip_prefix("pub enum "))
                .or_else(|| trimmed.strip_prefix("pub trait ")),
            Language::Go => trimmed.strip_prefix("type "),
            _ => None,
        };
        let Some(rest) = declared else {
            continue;
        };
        let name = rest
            .split(['(', '<', ' ', '{', ';'])
            .next()
            .unwrap_or_default();
        if name.len() > 1 && name.chars().next().is_some_and(char::is_uppercase) {
            return Some(name.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The production code
// ---------------------------------------------------------------------------

/// The marker a stub is left under, assembled rather than written: weed refuses
/// a work marker in production code, and the injector is production code.
fn work_marker() -> String {
    ['T', 'O', 'D', 'O'].iter().collect()
}

/// S1: a marker for work nobody did, in code that ships.
fn stub_production_code(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.production(lang), seed) {
        for function in rotate_owned(source.functions(), seed) {
            let Some(at) = source.body_first(function.first as usize) else {
                continue;
            };
            let indent = source.body_indent(function.first as usize);
            let marker = work_marker();
            let line = format!(
                "{indent}{}",
                lang.comment(&format!("{marker}: finish this"))
            );
            let text = with(source, at, &[line]);
            return Some(
                Mutation::new(
                    format!("a `{marker}` was left in `{}`", function.name),
                    &source.path,
                )
                .writing(&source.path, text)
                .at(at as u32 + 1, at as u32 + 1),
            );
        }
    }
    None
}

/// Whether a path is where a program starts, which is where printing is the
/// product rather than something left behind.
fn is_entry_point(path: &str) -> bool {
    let name = basename(path);
    let stem = name.rsplit_once('.').map_or(name, |(head, _)| head);
    matches!(stem, "main" | "__main__" | "cli")
}

/// S2: a failure arrives and nothing is done with it.
///
/// The rule is about a handler that came in with the change, so that is what is
/// planted: a statement wrapped in one that catches and says nothing, an empty
/// nil check under an assignment that could have failed, a fallible call turned
/// into a value that cannot fail.
fn swallow_a_failure(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.production(lang), seed) {
        let planted = match lang {
            Language::Rs => discard_a_failure(source),
            Language::Go => ignore_a_nil_check(source),
            _ => wrap_in_a_hollow_handler(source),
        };
        if planted.is_some() {
            return planted;
        }
    }
    None
}

/// A statement wrapped in a handler that catches the failure and says nothing.
fn wrap_in_a_hollow_handler(source: &Source) -> Option<Mutation> {
    let at = source.simple_statement()?;
    let statement = source.lines.get(at)?.clone();
    let indent = leading(&statement);
    let deeper = match source.lang {
        Language::Py => format!("{indent}    "),
        _ => format!("{indent}  "),
    };
    let wrapped = match source.lang {
        Language::Py => vec![
            format!("{indent}try:"),
            format!("{deeper}{}", statement.trim_start()),
            format!("{indent}except Exception:"),
            format!("{deeper}pass"),
        ],
        _ => vec![
            format!("{indent}try {{"),
            format!("{deeper}{}", statement.trim_start()),
            format!("{indent}}} catch (error) {{"),
            format!("{indent}}}"),
        ],
    };
    let handler = at as u32 + 3;
    let text = replace(source, at, &wrapped);
    Some(
        Mutation::new(
            "a handler was added that catches and says nothing",
            &source.path,
        )
        .writing(&source.path, text)
        .at(handler, handler),
    )
}

/// Go tests a failure against nil, and a check with nothing under it is the
/// failure going nowhere.
fn ignore_a_nil_check(source: &Source) -> Option<Mutation> {
    for function in source.functions() {
        for at in function.first as usize + 1..function.last as usize {
            let code = source.code.get(at)?;
            let trimmed = code.trim();
            if !trimmed.contains("err :=") && !trimmed.contains("err =") {
                continue;
            }
            if !trimmed.ends_with(')') {
                continue;
            }
            let indent = leading(source.lines.get(at)?);
            let planted = vec![format!("{indent}if err != nil {{"), format!("{indent}}}")];
            let check = at as u32 + 2;
            let text = with(source, at + 1, &planted);
            return Some(
                Mutation::new("a failure was checked and left where it fell", &source.path)
                    .writing(&source.path, text)
                    .at(check, check),
            );
        }
    }
    None
}

/// Rust hands a failure back rather than catching it, so what swallows one is a
/// call that turns it into a value with nothing asked.
fn discard_a_failure(source: &Source) -> Option<Mutation> {
    for (at, code) in source.code.iter().enumerate() {
        let trimmed = code.trim_end();
        if !trimmed.ends_with("?;") || trimmed.trim_start().starts_with("//") {
            continue;
        }
        let raw = source.lines.get(at)?;
        let mut lines = source.lines.clone();
        *lines.get_mut(at)? = raw.replacen("?;", ".unwrap_or_default();", 1);
        return Some(
            Mutation::new("a failure was turned into a default", &source.path)
                .writing(&source.path, joined(&lines))
                .at(at as u32 + 1, at as u32 + 1),
        );
    }
    None
}

/// S3: somebody watched the program run and left the line in.
fn leave_a_print(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.production(lang), seed) {
        if lang == Language::Go && !source.lines.iter().any(|line| line.contains("\"fmt\"")) {
            continue;
        }
        let functions = source.functions();
        for function in rotate_owned(functions.clone(), seed) {
            // The entry point is the program talking to whoever started it, and
            // a function written inside one is part of the same conversation.
            if function.name == "main" || is_entry_point(&source.path) {
                continue;
            }
            if functions.iter().any(|outer| {
                outer.name == "main" && outer.first < function.first && outer.last >= function.last
            }) {
                continue;
            }
            let Some(at) = source.body_first(function.first as usize) else {
                continue;
            };
            let indent = source.body_indent(function.first as usize);
            let printed = match lang {
                Language::Ts => "console.log('checking');".to_string(),
                Language::Py => "print(\"checking\")".to_string(),
                Language::Rs => "println!(\"checking\");".to_string(),
                Language::Go => "fmt.Println(\"checking\")".to_string(),
            };
            let text = with(source, at, &[format!("{indent}{printed}")]);
            return Some(
                Mutation::new(
                    format!("`{printed}` was left in `{}`", function.name),
                    &source.path,
                )
                .writing(&source.path, text)
                .at(at as u32 + 1, at as u32 + 1),
            );
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The rest of the repository
// ---------------------------------------------------------------------------

/// D1: what the program is built from changes inside a change about something
/// else.
fn change_a_manifest(lang: Language, tree: &Tree, _seed: u64) -> Option<Mutation> {
    let names: &[&str] = match lang {
        Language::Ts => &["package.json"],
        Language::Py => &["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"],
        Language::Rs => &["Cargo.toml"],
        Language::Go => &["go.mod"],
    };
    // The names are in the order the language's own packaging prefers them, and
    // so is the search: a repository that declares its dependencies in
    // `pyproject.toml` is not measured on the `setup.py` beside it.
    let mut manifests: Vec<&String> = tree
        .paths
        .iter()
        .filter(|path| names.contains(&basename(path)))
        .filter(|path| !tree.touched(path))
        .collect();
    manifests.sort_by_key(|path| {
        names
            .iter()
            .position(|name| *name == basename(path))
            .unwrap_or(names.len())
    });
    for path in manifests {
        let Some(text) = tree.text(path) else {
            continue;
        };
        let Some(bumped) = bump(&text) else {
            continue;
        };
        return Some(Mutation::new("a dependency pin was moved", path).writing(path, bumped));
    }
    None
}

/// The file with the first version it pins moved on by one patch.
fn bump(text: &str) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(ToString::to_string).collect();
    for line in &mut lines {
        let Some((at, end)) = version_span(line) else {
            continue;
        };
        let version = &line[at..end];
        let mut parts: Vec<String> = version.split('.').map(ToString::to_string).collect();
        let Some(last) = parts.last_mut() else {
            continue;
        };
        let Ok(patch) = last.parse::<u32>() else {
            continue;
        };
        *last = (patch + 1).to_string();
        let bumped = format!("{}{}{}", &line[..at], parts.join("."), &line[end..]);
        *line = bumped;
        let mut written = lines.join("\n");
        written.push('\n');
        return Some(written);
    }
    None
}

/// Where a `1.2.3` sits in a line, if one does.
fn version_span(line: &str) -> Option<(usize, usize)> {
    let bytes: Vec<char> = line.chars().collect();
    let mut at = 0;
    while at < bytes.len() {
        if !bytes[at].is_ascii_digit() || (at > 0 && bytes[at - 1].is_alphanumeric()) {
            at += 1;
            continue;
        }
        let mut end = at;
        let mut dots = 0;
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == '.') {
            if bytes[end] == '.' {
                dots += 1;
            }
            end += 1;
        }
        if dots == 2
            && !bytes
                .get(end)
                .is_some_and(|c| c.is_alphanumeric() || *c == '-')
        {
            return Some((at, end));
        }
        at = end + 1;
    }
    None
}

/// D2: one import, drawn against the direction the repository states.
fn cross_a_boundary(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let sources: Vec<&Source> = tree
        .sources(lang)
        .iter()
        .filter(|source| !tree.touched(&source.path) && layer_of(&source.path).is_some())
        .collect();
    for source in rotate(sources.clone(), seed) {
        let Some(here) = layer_of(&source.path) else {
            continue;
        };
        for other in rotate(sources.clone(), seed.wrapping_add(1)) {
            let Some(there) = layer_of(&other.path) else {
                continue;
            };
            if there == here || other.holds_tests() || source.holds_tests() {
                continue;
            }
            // Overlapping layers are one layer: a path that matches both globs
            // is claimed by whichever is named first, and the arrow disappears.
            if there.starts_with(&format!("{here}/")) || here.starts_with(&format!("{there}/")) {
                continue;
            }
            // An arrow that is already drawn is one this change would only be
            // standing on, so the file that gets the new import is one that has
            // never reached into the other layer.
            if source.code.iter().any(|line| line.contains(&there)) {
                continue;
            }
            let Some(statement) = import_of(lang, source, other, tree) else {
                continue;
            };
            let placement = placed(source, statement);
            let at = placement.at;
            let text = with(source, at, std::slice::from_ref(&placement.written));
            let config = format!(
                "[deps]\nlayers = {{ \"one\" = [\"{here}/**\"], \"two\" = [\"{there}/**\"] }}\nallow = []\n"
            );
            let mut planted = Mutation::new(
                format!("`{here}` was made to import `{there}`"),
                &source.path,
            )
            .writing(&source.path, text)
            .at(placement.first, at as u32 + 1);
            planted.config = Some(config);
            return Some(planted);
        }
    }
    None
}

/// The part of the repository a file belongs to, as a path prefix: the top
/// directory, or the one below it where the top is only a wrapper.
fn layer_of(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 {
        return None;
    }
    let first = segments[0];
    // A directory that only wraps the source is not a part of the program, so
    // the layer is the one below it. A file sitting directly in the wrapper
    // belongs to no layer at all: naming it one would make a layer that holds
    // every other layer inside it, and an arrow between those means nothing.
    if matches!(first, "src" | "lib" | "pkg" | "internal") {
        return (segments.len() > 2).then(|| format!("{first}/{}", segments[1]));
    }
    Some(first.to_string())
}

/// Where an import goes, and how it is written once it is there. A language
/// that declares its imports in one block takes the new one inside that block,
/// which is where anybody adding a dependency would put it.
fn placed(source: &Source, statement: String) -> Placement {
    if source.lang == Language::Go {
        if let Some(open) = source
            .code
            .iter()
            .position(|line| line.trim_end() == "import (")
        {
            if let Some(close) = source
                .code
                .iter()
                .skip(open)
                .position(|line| line.trim() == ")")
            {
                let named = statement.trim_start_matches("import ").trim().to_string();
                // The whole block is one declaration, so a finding about what
                // is inside it may land anywhere from the keyword down.
                return Placement {
                    at: open + close,
                    written: format!("\t{named}"),
                    first: open as u32 + 1,
                };
            }
        }
    }
    let at = source.last_import().map_or(0, |line| line + 1);
    Placement {
        at,
        written: statement,
        first: at as u32 + 1,
    }
}

/// Where a planted import goes and where a finding about it may land.
struct Placement {
    /// The index the line is written in front of.
    at: usize,
    written: String,
    /// The first line, one-based, a finding about it may point at.
    first: u32,
}

fn import_of(lang: Language, from: &Source, to: &Source, tree: &Tree) -> Option<String> {
    match lang {
        Language::Ts => Some(format!(
            "import '{}';",
            relative_module(&from.path, &to.path)
        )),
        Language::Py => Some(format!("import {}", dotted_module(&to.path))),
        Language::Rs => {
            let inside = to.path.strip_prefix("src/")?.strip_suffix(".rs")?;
            let module = inside.trim_end_matches("/mod").replace('/', "::");
            (!module.is_empty() && module != "lib" && module != "main")
                .then(|| format!("use crate::{module};"))
        }
        Language::Go => {
            let module = tree
                .text("go.mod")?
                .lines()
                .find_map(|line| line.strip_prefix("module ").map(ToString::to_string))?;
            let inside = directory(&to.path);
            (!inside.is_empty()).then(|| format!("import \"{}/{inside}\"", module.trim()))
        }
    }
}

/// X1: a credential is written into the tree. The value is random and belongs to
/// nobody; it is never printed, and it never leaves the temporary clone.
fn add_a_secret(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let source = rotate(tree.production(lang), seed).next()?;
    let at = source.last_import().map_or(0, |line| line + 1);
    let value = opaque(seed ^ 0x5eed, 40);
    let statement = match lang {
        Language::Ts => format!("const apiToken = '{value}';"),
        Language::Py => format!("API_TOKEN = \"{value}\""),
        Language::Rs => format!("const API_TOKEN: &str = \"{value}\";"),
        Language::Go => format!("const apiToken = \"{value}\""),
    };
    let text = with(source, at, &[statement]);
    Some(
        Mutation::new(
            "a credential was assigned to a name that says so",
            &source.path,
        )
        .writing(&source.path, text)
        .at(at as u32 + 1, at as u32 + 1),
    )
}

/// A value with no order in it, drawn from the seed so a run repeats.
fn opaque(seed: u64, width: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut state = seed | 1;
    (0..width)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let at = (state >> 33) as usize % ALPHABET.len();
            ALPHABET[at] as char
        })
        .collect()
}

/// X2: the change reaches a file nobody asked it to.
fn touch_outside_the_scope(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    if tree.changed.is_empty() {
        return None;
    }
    for source in rotate(tree.sources(lang).iter().collect::<Vec<_>>(), seed) {
        if tree.touched(&source.path) {
            continue;
        }
        let mut lines = source.lines.clone();
        lines.push(lang.comment("read while fixing something else"));
        let mut planted = Mutation::new(
            "a file the change was never scoped for was edited",
            &source.path,
        )
        .writing(&source.path, joined(&lines));
        // The scope is the sentence this commit was written under: the files it
        // touched, and nothing else.
        for path in &tree.changed {
            planted.arguments.push("--scope".to_string());
            planted.arguments.push(path.clone());
        }
        return Some(planted);
    }
    None
}

/// C1: the law the checks are run under is rewritten.
///
/// The subtlest shape is a line changed inside the section an instruction file
/// states its hard limits in, so that is planted where a repository carries
/// one. A repository that states no law is given the other shape the rule is
/// about, and the one every repository can receive: a hook git calls before a
/// commit, that passes whatever it is handed.
fn edit_a_guardrail(_lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let guardrails: Vec<&String> = tree
        .paths
        .iter()
        .filter(|path| is_guardrail(path))
        .filter(|path| !tree.touched(path))
        .collect();
    for path in rotate(guardrails, seed) {
        let Some(text) = tree.text(path) else {
            continue;
        };
        if let Some(planted) = widen_the_limits(path, &text) {
            return Some(planted);
        }
        if is_instructions(path) {
            // The rest of an instruction file is prose, and a rule about the
            // law does not fire on it.
            continue;
        }
        let mut written = text.clone();
        if !written.ends_with('\n') {
            written.push('\n');
        }
        written.push_str("# the checks are asked for less than they were\n");
        // The finding is about the file rather than about a line in it, so no
        // line is expected of it.
        return Some(Mutation::new("a guardrail file was edited", path).writing(path, written));
    }
    installed_hook(tree)
}

/// The guardrail files whose law lives in one section rather than in the whole
/// file, as weed reads them.
fn is_instructions(path: &str) -> bool {
    matches!(path, "AGENTS.md" | "CLAUDE.md")
}

fn is_guardrail(path: &str) -> bool {
    is_instructions(path)
        || path == "weed.toml"
        || path == ".gemini/settings.json"
        || path.starts_with(".codex/")
        || path.starts_with(".githooks/")
        || (path.starts_with(".claude/settings") && path.ends_with(".json"))
}

/// One more limit written into the section a document states its hard limits
/// in, where it states any: the limits are what the work is held to, and an
/// agent editing them is widening what it may do.
fn widen_the_limits(path: &str, text: &str) -> Option<Mutation> {
    if !is_instructions(path) {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let (at, rank) = lines.iter().enumerate().find_map(|(index, line)| {
        let rank = heading_rank(line)?;
        let title = line.trim_start_matches('#').trim().to_ascii_lowercase();
        title.contains("hard limits").then_some((index, rank))
    })?;
    let end = lines
        .iter()
        .enumerate()
        .skip(at + 1)
        .find(|(_, line)| heading_rank(line).is_some_and(|next| next <= rank))
        .map_or(lines.len(), |(next, _)| next);
    let mut written: Vec<String> = lines.iter().map(ToString::to_string).collect();
    written.insert(
        end,
        "- unless the change is small enough to be obvious".into(),
    );
    Some(
        Mutation::new("a line was written into the hard limits", path)
            .writing(path, joined(&written))
            .at(end as u32 + 1, end as u32 + 1),
    )
}

/// How deep a markdown heading sits, or `None` for a line that is not one.
fn heading_rank(line: &str) -> Option<usize> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let rest = &line[hashes..];
    (hashes > 0 && (rest.is_empty() || rest.starts_with(' '))).then_some(hashes)
}

/// A hook git runs before every commit, that asks nothing of what it is handed.
/// A repository with no law of its own written down can still be handed one
/// that says yes to everything.
fn installed_hook(tree: &Tree) -> Option<Mutation> {
    let path = ".githooks/pre-commit";
    if tree.paths.iter().any(|held| held == path) {
        return None;
    }
    Some(
        Mutation::new(
            "a hook that passes whatever it is given was installed",
            path,
        )
        .writing(
            path,
            "#!/bin/sh\n# nothing is asked of a commit here\nexit 0\n".to_string(),
        ),
    )
}

/// C3: the file that says how the checks run on a server is edited.
fn edit_a_workflow(_lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let workflows: Vec<&String> = tree
        .paths
        .iter()
        .filter(|path| is_workflow(path))
        .filter(|path| !tree.touched(path))
        .collect();
    for path in rotate(workflows, seed) {
        let Some(text) = tree.text(path) else {
            continue;
        };
        let mut written = text.clone();
        if !written.ends_with('\n') {
            written.push('\n');
        }
        written.push_str("# the checks run on the branch they are asked for\n");
        return Some(Mutation::new("a workflow was edited", path).writing(path, written));
    }
    None
}

fn is_workflow(path: &str) -> bool {
    path.starts_with(".github/workflows/") && (path.ends_with(".yml") || path.ends_with(".yaml"))
}

/// C2: the repository is told to look away from its own source.
fn broaden_an_ignore(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    let ignores: Vec<&String> = tree
        .paths
        .iter()
        .filter(|path| {
            matches!(
                basename(path),
                ".gitignore" | ".eslintignore" | ".prettierignore" | ".dockerignore"
            )
        })
        .filter(|path| !tree.touched(path))
        .collect();
    for path in rotate(ignores, seed) {
        let inside = directory(path);
        let covered: Vec<&Source> = tree
            .sources(lang)
            .iter()
            .filter(|source| source.path.starts_with(inside))
            .collect();
        let Some(source) = covered.first() else {
            continue;
        };
        let relative = source.path.strip_prefix(inside).unwrap_or(&source.path);
        let relative = relative.trim_start_matches('/');
        let pattern = match relative.split_once('/') {
            Some((head, _)) => format!("{head}/"),
            None => format!("*.{}", extension(relative)?),
        };
        let Some(text) = tree.text(path) else {
            continue;
        };
        if text
            .lines()
            .any(|line| line.trim() == pattern.trim_end_matches('/') || line.trim() == pattern)
        {
            continue;
        }
        let mut written = text.clone();
        if !written.ends_with('\n') {
            written.push('\n');
        }
        written.push_str(&pattern);
        written.push('\n');
        let at = written.lines().count() as u32;
        return Some(
            Mutation::new(format!("`{pattern}` was added to an ignore file"), path)
                .writing(path, written)
                .at(at, at),
        );
    }
    None
}

/// G1: both sides of a merge nobody finished.
fn commit_a_conflict(lang: Language, tree: &Tree, seed: u64) -> Option<Mutation> {
    for source in rotate(tree.sources(lang).iter().collect::<Vec<_>>(), seed) {
        let Some(at) = source
            .lines
            .iter()
            .enumerate()
            .find(|(at, line)| *at > 2 && !line.trim().is_empty())
            .map(|(at, _)| at)
        else {
            continue;
        };
        let held = source.lines.get(at)?.clone();
        let planted = vec![
            "<<<<<<< HEAD".to_string(),
            held.clone(),
            "=======".to_string(),
            held,
            ">>>>>>> theirs".to_string(),
        ];
        let width = planted.len() as u32;
        let text = replace(source, at, &planted);
        return Some(
            Mutation::new("a merge was left half finished", &source.path)
                .writing(&source.path, text)
                .at(at as u32 + 1, at as u32 + width),
        );
    }
    None
}

/// G2: something a build wrote arrives with the change.
fn add_a_blob(lang: Language, tree: &Tree, _seed: u64) -> Option<Mutation> {
    let path = format!("assets/{}-artifact.bin", lang.slug());
    if tree.paths.contains(&path) {
        return None;
    }
    // Bytes with no lines in them: the diff has nothing to show a reviewer.
    let bytes: String = (0..4096u32)
        .map(|at| char::from(u8::try_from(at % 251).unwrap_or_default()))
        .collect();
    Some(Mutation::new("a binary file was added", &path).writing(&path, bytes))
}

// ---------------------------------------------------------------------------
// Writing lines
// ---------------------------------------------------------------------------

fn joined(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// The file with lines put in front of an index.
fn with(source: &Source, at: usize, planted: &[String]) -> String {
    let mut lines = source.lines.clone();
    let at = at.min(lines.len());
    for (offset, line) in planted.iter().enumerate() {
        lines.insert(at + offset, line.clone());
    }
    joined(&lines)
}

/// The file with a run of lines taken out.
fn without(source: &Source, first: usize, last: usize) -> String {
    let mut lines = source.lines.clone();
    let last = last.min(lines.len().saturating_sub(1));
    lines.drain(first..=last);
    joined(&lines)
}

/// The file with one line replaced by several.
fn replace(source: &Source, at: usize, planted: &[String]) -> String {
    let mut lines = source.lines.clone();
    if at < lines.len() {
        lines.remove(at);
    }
    for (offset, line) in planted.iter().enumerate() {
        lines.insert(at + offset, line.clone());
    }
    joined(&lines)
}

fn leading(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Where the parenthesis opened at this index is closed.
fn closing(line: &str, open: usize) -> Option<usize> {
    let characters: Vec<char> = line.chars().collect();
    let mut depth = 0i32;
    for (at, character) in characters.iter().enumerate().skip(open) {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// The module specifier one file writes for another, relative, as the language
/// with no package name writes it.
fn relative_module(from: &str, to: &str) -> String {
    let from: Vec<&str> = directory(from)
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let to_directory: Vec<&str> = directory(to).split('/').filter(|s| !s.is_empty()).collect();
    let shared = from
        .iter()
        .zip(to_directory.iter())
        .take_while(|(one, other)| one == other)
        .count();
    let mut specifier = String::new();
    for _ in shared..from.len() {
        specifier.push_str("../");
    }
    if specifier.is_empty() {
        specifier.push_str("./");
    }
    for segment in &to_directory[shared..] {
        specifier.push_str(segment);
        specifier.push('/');
    }
    let name = basename(to);
    let stem = name.rsplit_once('.').map_or(name, |(head, _)| head);
    specifier.push_str(stem);
    specifier
}

/// The dotted name a module is imported by.
fn dotted_module(path: &str) -> String {
    path.trim_end_matches(".py")
        .trim_end_matches("/__init__")
        .trim_start_matches("src/")
        .replace('/', ".")
}

/// The same items, starting somewhere the seed decides, so two commits of one
/// repository are not both mutated in the same file.
fn rotate<T>(items: Vec<T>, seed: u64) -> std::vec::IntoIter<T> {
    let mut items = items;
    if !items.is_empty() {
        let at = (seed as usize) % items.len();
        items.rotate_left(at);
    }
    items.into_iter()
}

fn rotate_owned(items: Vec<Case>, seed: u64) -> std::vec::IntoIter<Case> {
    rotate(items, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_grows() {
        let line = "  const timeout = 500;";
        let (first, last) = number_span(line, line.find("timeout").unwrap_or_default())
            .expect("a number under the word");
        let (widened, before, after) = grown(line, first, last).expect("a number to grow");
        assert_eq!(widened, "  const timeout = 5000;");
        assert_eq!((before.as_str(), after.as_str()), ("500", "5000"));
    }

    #[test]
    fn a_year_in_a_string_is_no_wait() {
        // The code the injector reads has its strings blanked out, so the only
        // number left on this line is the one the wait is written with.
        let code = "  await waitFor(                 300);";
        let (first, last) =
            number_span(code, code.find("wait").unwrap_or_default()).expect("the wait");
        assert_eq!(&code[first..last], "300");
    }

    #[test]
    fn a_version_is_found_and_moved_on() {
        let bumped = bump("  \"left-pad\": \"^1.2.3\",\n").expect("a version to bump");
        assert_eq!(bumped, "  \"left-pad\": \"^1.2.4\",\n");
    }

    #[test]
    fn a_module_is_named_relative_to_the_file_that_imports_it() {
        assert_eq!(
            relative_module("test/unit/a.test.ts", "src/core/quota.ts"),
            "../../src/core/quota"
        );
        assert_eq!(relative_module("src/a.test.ts", "src/b.ts"), "./b");
    }

    #[test]
    fn an_opaque_value_repeats_for_a_seed() {
        assert_eq!(opaque(7, 40), opaque(7, 40));
        assert_ne!(opaque(7, 40), opaque(8, 40));
        assert_eq!(opaque(7, 40).len(), 40);
    }
}
