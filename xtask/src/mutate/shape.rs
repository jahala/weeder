//! Whether the shape a case claims is really in the tree, where the case says
//! it is.
//!
//! The injector reads a site out of somebody else's code, and a scanner that
//! does that sometimes reads a shape that was never there: a version string
//! that pins nothing, a call whose name only ends in the word a claim is
//! written with. A case planted on a site like that is worthless in both
//! directions. weed firing on it inflates recall, and weed staying quiet is
//! charged to the product as a miss for a shape it was never shown, which is
//! the worse of the two: it hides a broken injector behind a number that reads
//! like a gap in the binary.
//!
//! So the tree is read back after the case is written into it, by the same
//! scanner the injector finds its sites with and by nothing of weed's, and the
//! shape has to be there, on the line the case named. A case that is not
//! confirmed is unplantable: counted per rule and language, reported beside the
//! misses, and neither a hit nor a miss.
//!
//! What each rule is confirmed by is the shape the rule exists for, read the way
//! the language writes it: a suite that holds one case fewer, a case its runner
//! will not run, a claim that no longer names the failure, a version the line
//! pins somebody else's code at.

use super::inject::{self, Mutation};
use super::source::{basename, Case, Language, Source};
use super::tree::Tree;

/// Whether the shape this case claims is in the tree, at the site it names.
pub fn present(
    rule: &str,
    lang: Language,
    planted: &Mutation,
    tree: &Tree,
    before: &[(String, Option<String>)],
) -> bool {
    match rule {
        // A rename and a blob are about the files themselves rather than about
        // anything inside one, so neither is read as a site.
        "T7" => the_suite_left_the_runner(lang, planted, tree),
        "G2" => bytes_with_no_lines_arrived(planted, tree),
        _ => match Site::read(lang, planted, tree, before) {
            Some(site) => at_the_site(rule, &site, planted, tree),
            None => false,
        },
    }
}

fn at_the_site(rule: &str, site: &Site, planted: &Mutation, tree: &Tree) -> bool {
    match rule {
        "T1" => a_case_is_gone(site),
        "T2" => a_case_stopped_claiming(site),
        "T3" => a_case_is_turned_off(site),
        "T4" => a_slack_grew(site),
        "T5" => an_expectation_moved(site, tree),
        "T6" => a_claim_stopped_naming_the_failure(site),
        "M1" => a_double_stands_in_for_the_change(site, tree),
        "S1" => a_marker_for_work_is_in_shipped_code(site),
        "S2" => a_failure_goes_nowhere(site),
        "S3" => a_print_is_in_shipped_code(site),
        "D1" => a_dependency_pin_moved(site),
        "D2" => an_import_crosses_a_layer(site, tree),
        "X1" => a_credential_is_written_down(site),
        "X2" => the_change_reached_past_its_scope(site, planted),
        "C1" => a_guardrail_was_rewritten(site),
        "C2" => an_ignore_reaches_the_source(site, tree),
        "C3" => a_workflow_was_rewritten(site),
        "G1" => a_merge_was_left_half_finished(site),
        _ => false,
    }
}

/// One case's site: the file the shape was written into, read on both sides.
struct Site {
    lang: Language,
    path: String,
    before: Source,
    after: Source,
    /// The first line the shape was written on, zero-based, where it was
    /// written on lines at all.
    first: Option<usize>,
    /// The last of them.
    last: Option<usize>,
}

impl Site {
    fn read(
        lang: Language,
        planted: &Mutation,
        tree: &Tree,
        before: &[(String, Option<String>)],
    ) -> Option<Site> {
        let (path, _) = planted
            .writes
            .iter()
            .find(|(_, written)| written.is_some())?;
        let after = std::fs::read_to_string(tree.root.join(path)).ok()?;
        let held = before
            .iter()
            .find(|(named, _)| named == path)
            .and_then(|(_, text)| text.clone())
            .unwrap_or_default();
        Some(Site {
            lang,
            path: path.clone(),
            before: Source::read(path, lang, &held),
            after: Source::read(path, lang, &after),
            first: planted
                .lines
                .map(|(first, _)| first.saturating_sub(1) as usize),
            last: planted
                .lines
                .map(|(_, last)| last.saturating_sub(1) as usize),
        })
    }

    /// The masked code of the line the shape was written on: what is left of it
    /// once its strings and its comments are taken out.
    fn code(&self) -> Option<&String> {
        self.after.code.get(self.first?)
    }

    /// The line itself, as it is written.
    fn line(&self) -> Option<&String> {
        self.after.lines.get(self.first?)
    }

    /// Whether the line the shape was written on is inside the body of some
    /// function the file declares.
    fn inside_a_function(&self) -> bool {
        let Some(at) = self.first else {
            return false;
        };
        self.after
            .functions()
            .iter()
            .any(|held| (held.first as usize) < at && at <= held.last as usize)
    }

    /// Whether this file is code that ships rather than a suite.
    fn ships(&self) -> bool {
        !self.after.holds_tests() && !self.lang.collects_file(&self.path)
    }

    fn changed(&self) -> bool {
        self.before.lines != self.after.lines
    }
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// T1: the suite holds one case fewer than it did, and still holds one.
fn a_case_is_gone(site: &Site) -> bool {
    let held = site.before.cases().len();
    held > 1 && site.after.cases().len() + 1 == held
}

/// T2: one claim fewer, and every case that made them still declared.
fn a_case_stopped_claiming(site: &Site) -> bool {
    site.before.assertions().len() == site.after.assertions().len() + 1
        && site.before.cases().len() == site.after.cases().len()
}

/// T3: a case its runner will not run, marked at the line the case named.
fn a_case_is_turned_off(site: &Site) -> bool {
    let Some(at) = site.first else {
        return false;
    };
    site.after
        .cases()
        .iter()
        .filter(|case| (case.first as usize).abs_diff(at) <= 1)
        .any(|case| is_turned_off(&site.after, case))
}

/// Whether the runner passes this case over, by the marker the runner reads.
fn is_turned_off(source: &Source, case: &Case) -> bool {
    let first = case.first as usize;
    match source.lang {
        Language::Ts => source
            .code
            .get(first)
            .is_some_and(|code| code.contains(".skip") || code.contains(".todo")),
        // The decorators a case carries are the lines above its declaration.
        Language::Py => (0..first)
            .rev()
            .map(|at| source.code.get(at).map_or("", String::as_str).trim())
            .take_while(|code| code.starts_with('@') || code.is_empty())
            .any(|code| code.contains(".skip")),
        // Between the attribute that collects the case and the function it
        // collects.
        Language::Rs => source
            .code
            .iter()
            .skip(first)
            .take_while(|code| !code.contains("fn "))
            .any(|code| code.contains("#[ignore]")),
        Language::Go => (first + 1..=case.last as usize).any(|at| {
            source
                .code
                .get(at)
                .is_some_and(|code| code.contains(".Skip("))
        }),
    }
}

/// T4: the number under the slack word is ten times what it was, on a line that
/// makes no claim.
fn a_slack_grew(site: &Site) -> bool {
    let Some(at) = site.first else {
        return false;
    };
    if site.after.assertions().contains(&(at as u32)) {
        return false;
    }
    let (Some(now), Some(held)) = (site.after.code.get(at), site.before.code.get(at)) else {
        return false;
    };
    let (Some(grown), Some(was)) = (slack_number(now), slack_number(held)) else {
        return false;
    };
    was > 0.0 && (grown - was * 10.0).abs() <= was * 1e-9
}

/// The number written under the first slack word on a line, where one is.
fn slack_number(code: &str) -> Option<f64> {
    let word = inject::slack_word(code)?;
    let (first, last) = inject::number_span(code, word)?;
    code.chars()
        .take(last)
        .skip(first)
        .collect::<String>()
        .parse::<f64>()
        .ok()
}

/// T5: a recorded expectation was rewritten in a change that also rewrote the
/// code it records.
fn an_expectation_moved(site: &Site, tree: &Tree) -> bool {
    inject::is_expectation(&site.path)
        && site.changed()
        && tree
            .production(site.lang)
            .iter()
            .any(|source| tree.touched(&source.path))
}

/// T6: a claim about a failure that named one and no longer does, inside the
/// body of a case rather than on the line that declares one.
fn a_claim_stopped_naming_the_failure(site: &Site) -> bool {
    let Some(at) = site.first else {
        return false;
    };
    let inside = site
        .after
        .cases()
        .iter()
        .any(|case| (case.first as usize) < at && at <= case.last as usize);
    if !inside {
        return false;
    }
    let (Some(now), Some(held)) = (site.after.code.get(at), site.before.code.get(at)) else {
        return false;
    };
    let (Some(written), Some(was)) = (site.after.lines.get(at), site.before.lines.get(at)) else {
        return false;
    };
    names_no_failure(site.lang, now, written) && named_a_failure(site.lang, held, was)
}

/// Whether a claim about a failure says which failure it is about.
fn named_a_failure(lang: Language, code: &str, raw: &str) -> bool {
    match lang {
        Language::Ts => thrown(code, raw).is_some_and(|named| !named.trim().is_empty()),
        Language::Py => raised(code).is_some_and(|named| {
            let named = named.trim();
            !named.is_empty() && named != "Exception" && named != "BaseException"
        }),
        Language::Rs => code.contains("Err("),
        Language::Go => code.contains("errors.Is(") || code.contains("errors.As("),
    }
}

/// Whether a claim about a failure has stopped saying which one.
fn names_no_failure(lang: Language, code: &str, raw: &str) -> bool {
    match lang {
        Language::Ts => thrown(code, raw).is_some_and(|named| named.trim().is_empty()),
        Language::Py => raised(code).is_some_and(|named| {
            let named = named.trim();
            named == "Exception" || named == "BaseException"
        }),
        Language::Rs => code.contains(".is_err()") && !code.contains("Err("),
        Language::Go => code.trim() == "if err == nil {",
    }
}

/// What a claim says the code throws, read off the line rather than off the
/// masked code: a message the claim is written around is a string, and masking
/// takes strings out.
fn thrown(code: &str, raw: &str) -> Option<String> {
    let call = [".toThrowError(", ".toThrow("]
        .into_iter()
        .find(|call| code.contains(call))?;
    let at = char_find(code, call)? + call.chars().count();
    let end = closing_at(code, at - 1)?;
    Some(raw.chars().take(end).skip(at).collect())
}

/// What a claim says is raised, read off the masked code: the kind is written
/// first and a message beside it is a string.
fn raised(code: &str) -> Option<String> {
    let at = char_find(code, "raises(")? + "raises(".chars().count();
    let end = closing_at(code, at - 1)?;
    Some(code.chars().take(end).skip(at).collect())
}

/// T7: the file its runner collected is gone, and what stands in its place
/// still holds the cases and is collected by nobody.
fn the_suite_left_the_runner(lang: Language, planted: &Mutation, tree: &Tree) -> bool {
    let removed = planted
        .writes
        .iter()
        .find_map(|(path, written)| written.is_none().then_some(path));
    let written = planted
        .writes
        .iter()
        .find_map(|(path, written)| written.is_some().then_some(path));
    let (Some(removed), Some(written)) = (removed, written) else {
        return false;
    };
    if !lang.collects_file(removed) || lang.collects_file(written) {
        return false;
    }
    if tree.root.join(removed).exists() {
        return false;
    }
    std::fs::read_to_string(tree.root.join(written))
        .map(|text| !Source::read(written, lang, &text).cases().is_empty())
        .unwrap_or_default()
}

/// M1: a double of something this very commit changed, standing in a suite.
fn a_double_stands_in_for_the_change(site: &Site, tree: &Tree) -> bool {
    let Some(code) = site.code() else {
        return false;
    };
    let Some(raw) = site.line() else {
        return false;
    };
    if !site.after.holds_tests() {
        return false;
    }
    let spelled = code.to_ascii_lowercase();
    if !["mock", "patch", "stub", "fake", "double"]
        .iter()
        .any(|word| spelled.contains(word))
    {
        return false;
    }
    tree.production(site.lang)
        .iter()
        .filter(|source| tree.touched(&source.path))
        .any(|source| names_the_unit(raw, source))
}

/// Whether a line names a file of the program, by the name the file is imported
/// under or by a type it makes available.
fn names_the_unit(raw: &str, source: &Source) -> bool {
    let name = basename(&source.path);
    let stem = name.rsplit_once('.').map_or(name, |(head, _)| head);
    (!stem.is_empty() && raw.contains(stem))
        || inject::exported_type(source).is_some_and(|named| raw.contains(&named))
}

// ---------------------------------------------------------------------------
// The production code
// ---------------------------------------------------------------------------

/// S1: a marker for work nobody did, written as a comment inside a function of
/// code that ships.
fn a_marker_for_work_is_in_shipped_code(site: &Site) -> bool {
    let (Some(raw), Some(code)) = (site.line(), site.code()) else {
        return false;
    };
    let marker = inject::work_marker();
    // The marker is in the line and not in its code, which is what makes it a
    // comment rather than something the program does.
    site.ships() && raw.contains(&marker) && !code.contains(&marker) && site.inside_a_function()
}

/// S2: a failure arrives at a handler that does nothing with it.
fn a_failure_goes_nowhere(site: &Site) -> bool {
    let Some(at) = site.first else {
        return false;
    };
    let Some(code) = site.after.code.get(at) else {
        return false;
    };
    let trimmed = code.trim();
    let next = site
        .after
        .code
        .get(at + 1)
        .map_or("", String::as_str)
        .trim();
    let hollow = match site.lang {
        Language::Ts => trimmed.contains("catch") && trimmed.ends_with('{') && next == "}",
        Language::Py => trimmed.starts_with("except") && trimmed.ends_with(':') && next == "pass",
        Language::Go => trimmed == "if err != nil {" && next == "}",
        Language::Rs => trimmed.contains(".unwrap_or_default()"),
    };
    hollow && site.inside_a_function()
}

/// S3: a line that prints, left inside a function of code that ships.
fn a_print_is_in_shipped_code(site: &Site) -> bool {
    let Some(code) = site.code() else {
        return false;
    };
    let printed = match site.lang {
        Language::Ts => "console.log(",
        Language::Py => "print(",
        Language::Rs => "println!(",
        Language::Go => "fmt.Print",
    };
    site.ships() && code.contains(printed) && site.inside_a_function()
}

// ---------------------------------------------------------------------------
// The rest of the repository
// ---------------------------------------------------------------------------

/// D1: one line of a manifest moved, and that line pins a dependency.
fn a_dependency_pin_moved(site: &Site) -> bool {
    if !inject::manifest_names(site.lang).contains(&basename(&site.path)) {
        return false;
    }
    if site.before.lines.len() != site.after.lines.len() {
        return false;
    }
    let mut moved = site
        .before
        .lines
        .iter()
        .zip(site.after.lines.iter())
        .filter(|(held, now)| held != now);
    let Some((held, now)) = moved.next() else {
        return false;
    };
    moved.next().is_none()
        && inject::dependency_pin(held).is_some()
        && inject::dependency_pin(now).is_some()
}

/// D2: an import, written where the file declares them, naming a file of
/// another part of the repository.
fn an_import_crosses_a_layer(site: &Site, tree: &Tree) -> bool {
    let Some(at) = site.last else {
        return false;
    };
    if site.after.holds_tests() {
        return false;
    }
    let Some(here) = inject::layer_of(&site.path) else {
        return false;
    };
    if site.after.last_import().is_none_or(|last| at > last) {
        return false;
    }
    let Some(raw) = site.after.lines.get(at) else {
        return false;
    };
    let Some(named) = module_named(site.lang, raw) else {
        return false;
    };
    reached(tree, site.lang, &named).is_some_and(|there| there != here)
}

/// The module a statement names: what stands in its quotes, or what follows the
/// word the language imports with.
fn module_named(lang: Language, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if let Some((_, rest)) = trimmed.split_once(['"', '\'']) {
        if let Some((quoted, _)) = rest.split_once(['"', '\'']) {
            return (!quoted.is_empty()).then(|| quoted.to_string());
        }
    }
    let keyword = match lang {
        Language::Ts | Language::Py | Language::Go => "import ",
        Language::Rs => "use ",
    };
    let rest = trimmed.strip_prefix(keyword)?;
    let named = rest.trim_end_matches(';').trim();
    (!named.is_empty()).then(|| named.to_string())
}

/// The part of the repository a named module lives in, where the tree holds a
/// file that answers to the name.
///
/// The longest tail of the name a file answers to is the one that decides: two
/// files of a repository share a last segment often enough that matching on one
/// alone would name the wrong part of it.
fn reached(tree: &Tree, lang: Language, named: &str) -> Option<String> {
    let segments = segments(lang, named);
    for take in (1..=segments.len()).rev() {
        let tail = segments[segments.len() - take..].join("/");
        if let Some(path) = tree.paths.iter().find(|path| answers_to(path, &tail)) {
            return inject::layer_of(path);
        }
    }
    None
}

/// A module name in the parts the language writes it in.
fn segments(lang: Language, named: &str) -> Vec<String> {
    let split: Vec<&str> = match lang {
        Language::Py => named.split('.').collect(),
        Language::Rs => named.split("::").collect(),
        Language::Ts | Language::Go => named.split('/').collect(),
    };
    split
        .into_iter()
        .map(str::trim)
        .filter(|segment| !segment.is_empty() && !matches!(*segment, "." | ".." | "crate" | "self"))
        .map(ToString::to_string)
        .collect()
}

/// Whether a file of the tree is what a tail of a module name points at: the
/// file itself, or the directory a package is written in.
fn answers_to(path: &str, tail: &str) -> bool {
    let stem = path.rsplit_once('.').map_or(path, |(head, _)| head);
    // A package is named by its directory where the file inside it is the one
    // the language reads by default.
    let package = stem.trim_end_matches("/mod").trim_end_matches("/index");
    [stem, package, super::source::directory(path)]
        .iter()
        .any(|held| *held == tail || held.ends_with(&format!("/{tail}")))
}

/// X1: a name that says it holds a credential, given a value nobody can read
/// anything out of.
fn a_credential_is_written_down(site: &Site) -> bool {
    let (Some(code), Some(raw)) = (site.code(), site.line()) else {
        return false;
    };
    let spelled = code.to_ascii_lowercase();
    let named = ["token", "secret", "key", "password", "passwd", "credential"]
        .iter()
        .any(|word| spelled.contains(word));
    named
        && code.contains('=')
        && literal(raw).is_some_and(|value| {
            value.chars().count() >= 20 && !value.chars().any(char::is_whitespace)
        })
}

/// The first thing in quotes on a line.
fn literal(raw: &str) -> Option<String> {
    let (_, rest) = raw.split_once(['"', '\'', '`'])?;
    let (held, _) = rest.split_once(['"', '\'', '`'])?;
    Some(held.to_string())
}

/// X2: the run was told which paths the change was for, and this file is not
/// one of them.
fn the_change_reached_past_its_scope(site: &Site, planted: &Mutation) -> bool {
    let mut scoped = Vec::new();
    let mut arguments = planted.arguments.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--scope" {
            if let Some(path) = arguments.next() {
                scoped.push(path.clone());
            }
        }
    }
    !scoped.is_empty() && !scoped.contains(&site.path) && site.changed()
}

/// C1: a file the checks are run under, rewritten. Where the case wrote into
/// the section a document states its hard limits in, the line has to be in it.
fn a_guardrail_was_rewritten(site: &Site) -> bool {
    if !inject::is_guardrail(&site.path) || !site.changed() {
        return false;
    }
    let Some(at) = site.first else {
        return true;
    };
    inject::hard_limits(&site.after.text()).is_some_and(|(first, end)| first < at && at < end)
}

/// C2: a pattern the ignore file did not carry, that reaches a source file the
/// repository holds.
fn an_ignore_reaches_the_source(site: &Site, tree: &Tree) -> bool {
    let Some(at) = site.first else {
        return false;
    };
    let Some(pattern) = site.after.lines.get(at).map(|line| line.trim().to_string()) else {
        return false;
    };
    if pattern.is_empty() || site.before.lines.iter().any(|held| held.trim() == pattern) {
        return false;
    }
    let inside = super::source::directory(&site.path);
    tree.sources(site.lang)
        .iter()
        .any(|source| covers(inside, &pattern, &source.path))
}

/// Whether an ignore pattern written in this directory reaches this file.
fn covers(inside: &str, pattern: &str, path: &str) -> bool {
    let Some(relative) = path.strip_prefix(inside) else {
        return false;
    };
    let relative = relative.trim_start_matches('/');
    match pattern.strip_prefix("*.") {
        Some(extension) => relative.ends_with(&format!(".{extension}")),
        None => relative.starts_with(pattern.trim_end_matches('/')),
    }
}

/// C3: the file that says how the checks run on a server, rewritten.
fn a_workflow_was_rewritten(site: &Site) -> bool {
    inject::is_workflow(&site.path) && site.changed()
}

/// G1: both sides of a merge and the markers between them, where the case said.
fn a_merge_was_left_half_finished(site: &Site) -> bool {
    let (Some(first), Some(last)) = (site.first, site.last) else {
        return false;
    };
    let held: Vec<&String> = site
        .after
        .lines
        .iter()
        .skip(first)
        .take(last.saturating_sub(first) + 1)
        .collect();
    let opened = held.iter().position(|line| line.starts_with("<<<<<<<"));
    let between = held.iter().position(|line| line.starts_with("======="));
    let closed = held.iter().position(|line| line.starts_with(">>>>>>>"));
    matches!((opened, between, closed), (Some(one), Some(two), Some(three)) if one < two && two < three)
}

/// G2: bytes a diff has no lines to show.
fn bytes_with_no_lines_arrived(planted: &Mutation, tree: &Tree) -> bool {
    let Some((path, _)) = planted.writes.iter().find(|(_, written)| written.is_some()) else {
        return false;
    };
    std::fs::read(tree.root.join(path))
        .map(|bytes| bytes.len() > 512 && bytes.contains(&0))
        .unwrap_or_default()
}

/// Where a run of characters begins, counted in characters, so a position found
/// in a masked line can be used on the line it was masked from.
fn char_find(text: &str, wanted: &str) -> Option<usize> {
    text.find(wanted).map(|byte| text[..byte].chars().count())
}

/// Where the parenthesis opened at this character is closed, counted in
/// characters.
fn closing_at(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (at, character) in text.chars().enumerate().skip(open) {
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
