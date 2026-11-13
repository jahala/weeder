//! S1, a stub or a work marker reached production code.
//!
//! Three shapes, all of them a line admitting the work is not done.
//!
//! A work marker, one of the shouted words in [`WORK_MARKERS`], written as a
//! word of its own, in code or in a comment but never inside a string, where it
//! is a message the program hands out rather than a note to the author. A
//! marker counts where the change wrote it rather than carried it over: a
//! comment the change fixed a typo in, or moved word for word, carried its
//! marker and added none, while a marker taken out in one place and a different
//! one written in another is new work (#181).
//!
//! An unfinished signal: the token a language keeps for a body nobody wrote
//! (`todo!`, `unimplemented!`, `NotImplementedError`), or a throw, a raise or a
//! panic whose message, read as letters alone, says the same thing. An error
//! of that name is a signal only where a body raises, throws or returns it;
//! the code that imports it, checks a failure against it or catches it is
//! answering for a placeholder written somewhere else.
//!
//! A body that does nothing: a function whose whole body is the language's word
//! for "no statements", `pass`, `...`, a bare return, a return of the language's
//! empty value. The reader's outline says where each body starts and ends, the
//! change says the body is new, and the finding lands on the statement itself.
//!
//! One empty body is not a stub at all. Python states a shape by writing a
//! signature and no body, so the methods of a protocol and of an abstract base
//! are types rather than work left over: there is nothing there to finish, and
//! writing something would be putting an implementation where a declaration
//! belongs. weeder reads the class the method is written in and lets those be.
//!
//! An unfinished signal written inside a type is the same sentence about two
//! different things. In a leaf it is a stub, and a caller reaching it gets the
//! failure. In a base it is the contract, and what fills it in is somewhere else
//! in the tree: the six strategy classes an audit reported were six bases, each
//! written out by a subclass in another file. So weeder asks the repository
//! before it reports one. A type built on that base which writes the method is
//! the answer, and the finding never happens; a subclass a suite wrote is not,
//! because it stands in for nothing a caller reaches. Where nothing writes it
//! the finding stands and says what was looked for, and where the base is
//! stated through an entry file it warns instead, because a contract a package
//! exports is one another repository may be filling in.

use std::collections::{BTreeMap, BTreeSet};

use crate::core::change::Change;
use crate::core::classify::{FileKind, Lang};
use crate::core::finding::{Detection, Level, Message, Region, Stamp};
use crate::core::hierarchy::{self, Hierarchy};
use crate::core::read::{Definition, DefinitionKind, Outline};
use crate::core::rules::check::idiom;
use crate::core::rules::check::Judgement;
use crate::core::syntax::{words, Mask, Word};

/// The words that mark work left for later, written as they are meant to be
/// found: in capitals, so the same word in a sentence is prose and the shouted
/// one is a marker. weeder does not carry what it refuses, which is why these
/// three are written here and nowhere else in this repository.
const WORK_MARKERS: &[&str] = &["TODO", "FIXME", "XXX"];

/// The errors a language names a body nobody wrote with. Code that imports one,
/// checks a failure against one or catches one is answering for a placeholder
/// somewhere else (#94), so a name counts only where a body hands it out: right
/// after a keyword in [`HANDING_OUT`], with nothing between them but the words
/// in [`BETWEEN`] and the path it is reached through.
const UNFINISHED_TOKENS: &[&str] = &["NotImplemented", "NotImplementedError"];

/// The macros Rust keeps for a body nobody wrote. Each is an ordinary name for
/// an ordinary thing too, in Rust and in every other language: `todo !== null`
/// in TypeScript is a variable compared with nothing (#95). So these count in
/// Rust alone, and only as a macro called: the name, a `!` written against it,
/// and the bracket that opens the call.
const UNFINISHED_MACROS: &[&str] = &["todo", "unimplemented"];

/// What may open a macro's arguments, right after its `!`.
const MACRO_OPENERS: &[char] = &['(', '[', '{'];

/// The keywords that hand a failure to the caller rather than returning one.
/// A message given to one of these is the code describing itself.
const RAISING: &[&str] = &["throw", "raise", "panic"];

/// The shell has no exception to raise: a script fails by exiting, and a
/// function by returning, with a status. A message written on the same line as
/// either one is the script describing why it stopped.
const SHELL_RAISING: &[&str] = &["exit", "return"];

/// The keywords a body hands its answer out with: the ones that raise, and the
/// one that returns.
const HANDING_OUT: &[&str] = &["throw", "raise", "panic", "return"];

/// The words that may stand between a keyword handing an answer out and the
/// error it hands: `throw new`, `return Err(`, Go's `return nil,`.
const BETWEEN: &[&str] = &["new", "Err", "nil"];

/// A message that says the work is not done, with everything but its letters
/// taken out, so spelling and punctuation stop mattering.
const UNFINISHED_PHRASES: &[&str] = &["notimplemented", "notyetimplemented", "unimplemented"];

/// The bases a Python class is given when its methods are declarations. A
/// protocol states the shape a caller may rely on and an abstract base states
/// what a subclass has to write; neither one carries an implementation, which
/// is why an empty body under either is the whole of what was meant.
const DECLARING_BASES: &[&str] = &["Protocol", "ABC", "ABCMeta"];

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        for found in stubs(change) {
            if let Some(contract) = &found.contract {
                if judged.hierarchy.overrides(&contract.base, &contract.method) {
                    continue;
                }
            }
            findings.push(finding(path, &found, judged.hierarchy));
        }
    }
    findings
}

/// The types this change leaves an unfinished signal inside.
///
/// The face reads the tree for these and for nothing else. Walking a repository
/// is the one expensive thing a check can do, and a change that leaves no
/// unfinished signal in a method has nothing to ask it.
#[must_use]
pub fn types_in_question(changes: &[Change]) -> BTreeSet<String> {
    changes
        .iter()
        .flat_map(stubs)
        .filter_map(|found| found.contract.map(|contract| contract.base))
        .collect()
}

/// One stub a change added, and the contract it sits in where it sits in one.
struct Found {
    line: u32,
    stub: Stub,
    contract: Option<Contract>,
}

/// A method another type could write, named by the type that states it. This is
/// the question weeder takes to the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Contract {
    base: String,
    method: String,
}

/// Every stub one changed file added, in line order and one to a line.
fn stubs(change: &Change) -> Vec<Found> {
    if !change.after.is(FileKind::Prod) {
        return Vec::new();
    }
    let lang = change.after.lang();
    let mask = change.after.mask();
    let gained = gained_markers(change);
    let mut found: Vec<(u32, Stub)> = change
        .added()
        .filter_map(|(line, _)| {
            let stub = match gained.get(&line) {
                Some(word) => Stub::marker(word),
                None => signal_on(lang, mask, line)?,
            };
            Some((line, stub))
        })
        .collect();
    // A body that does nothing is found through the outline rather than line by
    // line, so it arrives out of order and may land on a line a marker already
    // spoke for.
    for (line, stub) in empty_bodies(lang, mask, change) {
        if !found.iter().any(|(reported, _)| *reported == line) {
            found.push((line, stub));
        }
    }
    found.sort_by_key(|(line, _)| *line);
    found
        .into_iter()
        .map(|(line, stub)| Found {
            contract: stub
                .signal
                .then(|| contract_at(&change.after.outline, line))
                .flatten(),
            line,
            stub,
        })
        .collect()
}

/// The contract a line sits in: the method it is written in and the type that
/// states it, where the line is written inside both.
///
/// The method has to be the type's own. A function written inside a method is
/// nobody's contract, and naming the type it happens to sit in would be weeder
/// answering a question it was never asked.
fn contract_at(outline: &Outline, line: u32) -> Option<Contract> {
    let method = hierarchy::enclosing_function(outline, line)?;
    let base = hierarchy::enclosing_type(outline, method)?;
    base.children
        .iter()
        .any(|child| child.start_line == method.start_line && child.name == method.name)
        .then(|| Contract {
            base: base.name.clone(),
            method: method.name.clone(),
        })
}

/// The work markers a line holds outside its literals, in the order it writes
/// them, each as often as it is written.
fn markers(mask: &Mask, line: u32) -> Vec<&'static str> {
    words(&mask.outside_literals(line))
        .into_iter()
        .filter_map(|word| {
            WORK_MARKERS
                .iter()
                .copied()
                .find(|marker| *marker == word.text)
        })
        .collect()
}

/// The added lines that write a work marker the change did not only carry
/// over, each with the first such marker it writes.
///
/// A marker is carried over by its own line and by nothing else. A line moved
/// word for word carried every marker it holds. A line that rewrote another
/// carried that line's markers, where the rewrite kept at least half of its
/// words: a typo fixed in a comment, a word changed beside the marker. A
/// rewrite that kept fewer is a new line in the old one's place, and a marker
/// on it is new, as is any marker on a line that rewrote nothing. Markers are
/// never netted across a file by their word, because a change that took one
/// marker out and wrote a different one elsewhere added work nobody wrote, and
/// the count would call that nothing (#181).
fn gained_markers(change: &Change) -> BTreeMap<u32, &'static str> {
    let spaced = |text: &str| text.split_whitespace().collect::<Vec<&str>>().join(" ");
    let removed: BTreeMap<u32, &str> = change.removed().collect();
    let taken_out: BTreeSet<String> = removed.values().map(|text| spaced(text)).collect();
    let rewrote: BTreeMap<u32, u32> = change
        .replacements()
        .iter()
        .map(|replacement| (replacement.new_line, replacement.old_line))
        .collect();

    let mut found = BTreeMap::new();
    for (line, text) in change.added() {
        let carries = markers(change.after.mask(), line);
        if carries.is_empty() || taken_out.contains(&spaced(text)) {
            continue;
        }
        // A line that rewrote no other, or rewrote it beyond recognition,
        // carried nothing over: no partner is an answer here, not a failure.
        let carried = rewrote
            .get(&line)
            .filter(|old| {
                removed
                    .get(*old)
                    .is_some_and(|before| kept_most_words(before, text))
            })
            .map(|old| markers(change.before.mask(), *old))
            .unwrap_or_default();
        let count = |held: &[&'static str], word: &str| {
            held.iter().filter(|marker| **marker == word).count()
        };
        if let Some(word) = carries
            .iter()
            .copied()
            .find(|word| count(&carries, word) > count(&carried, word))
        {
            found.insert(line, word);
        }
    }
    found
}

/// Whether a rewritten line kept at least half of the words of the line it
/// replaced, counted over both: the words the two share, against every word
/// either holds.
fn kept_most_words(before: &str, after: &str) -> bool {
    let old: BTreeSet<&str> = before.split_whitespace().collect();
    let new: BTreeSet<&str> = after.split_whitespace().collect();
    let shared = old.intersection(&new).count();
    let either = old.union(&new).count();
    either > 0 && shared * 2 >= either
}

/// The unfinished signal an added line carries on its own, without the
/// outline's help: the language's word for a body nobody wrote, or a failure
/// that says as much.
fn signal_on(lang: Lang, mask: &Mask, line: u32) -> Option<Stub> {
    let code = mask.code(line);
    let spoken = words(&code);
    if let Some(word) = spoken.iter().enumerate().find_map(|(at, word)| {
        let handed = UNFINISHED_TOKENS.contains(&word.text)
            && !word.is_member()
            && handed_out(&code, &spoken[..at]);
        let macro_call = lang == Lang::Rust
            && UNFINISHED_MACROS.contains(&word.text)
            && is_macro_call(&code, word);
        (handed || macro_call).then_some((word, macro_call))
    }) {
        // A macro is quoted back with the `!` that makes it one, so the reader
        // is shown the thing they wrote rather than the name inside it.
        let (word, macro_call) = word;
        let written = if macro_call {
            format!("{}!", word.text)
        } else {
            word.text.to_string()
        };
        return Some(Stub::token(&written));
    }

    let failing = match lang {
        Lang::Bash => SHELL_RAISING,
        _ => RAISING,
    };
    let raising = spoken.iter().any(|word| failing.contains(&word.text));
    if raising && says_unfinished(&mask.literals(line)) {
        return Some(Stub::announcement());
    }
    None
}

/// Whether a word is a macro called: a `!` written against the name, and then
/// the bracket that opens its arguments, with nothing but spaces between. `!=`
/// and `!==` are a comparison.
fn is_macro_call(code: &str, word: &Word<'_>) -> bool {
    code[word.end..]
        .strip_prefix('!')
        .and_then(|rest| rest.trim_start().chars().next())
        .is_some_and(|opener| MACRO_OPENERS.contains(&opener))
}

/// Whether the words written before an error's name on its line hand it out:
/// read back from the name, a keyword in [`HANDING_OUT`] reached across nothing
/// but the words in [`BETWEEN`] and a path's segments (`Error::`). An import,
/// an `instanceof`, an `isinstance(` and a `catch` reach no such keyword, and
/// neither does `return error instanceof NotImplemented`, whose answer is the
/// check rather than the error.
fn handed_out(code: &str, before: &[Word<'_>]) -> bool {
    for word in before.iter().rev() {
        if HANDING_OUT.contains(&word.text) {
            return true;
        }
        let segment = code[word.end..].trim_start().starts_with("::");
        if !(BETWEEN.contains(&word.text) || segment) {
            return false;
        }
    }
    false
}

/// Whether a message, read as letters alone, says the work is not done.
fn says_unfinished(text: &str) -> bool {
    let letters: String = text
        .chars()
        .filter(|character| character.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    UNFINISHED_PHRASES
        .iter()
        .any(|phrase| letters.contains(phrase))
}

/// Every function the change added a line to whose whole body does nothing,
/// with the added line the finding lands on.
fn empty_bodies(lang: Lang, mask: &Mask, change: &Change) -> Vec<(u32, Stub)> {
    let added: Vec<u32> = change.added().map(|(line, _)| line).collect();
    let mut found = Vec::new();
    for definition in change.after.outline.flatten() {
        if definition.kind != DefinitionKind::Function {
            continue;
        }
        let Some((line, statement)) = does_nothing(lang, mask, definition) else {
            continue;
        };
        if !added.iter().any(|added| definition.spans(*added)) {
            continue;
        }
        if declares_a_type(lang, mask, &change.after.outline, definition) {
            continue;
        }
        found.push((line, Stub::empty_body(&definition.name, &statement)));
    }
    found
}

/// The one statement a function's body holds and the line it is written on,
/// where that statement does nothing at all. A body with anything else in it,
/// and a body with nothing in it at all, both answer `None`: this rule is about
/// a function that answers, and an empty one has not been written yet.
fn does_nothing(lang: Lang, mask: &Mask, definition: &Definition) -> Option<(u32, String)> {
    let span = idiom::Block {
        first: definition.start_line,
        last: definition.end_line,
    };
    let statements = idiom::statements(&idiom::body(lang, mask, span)?);
    let [only] = statements.as_slice() else {
        return None;
    };
    if !idiom::does_nothing(&statements) {
        return None;
    }
    // Where the statement is written on a line of its own, that is the line
    // the finding points at; a body written on the declaration's own line has
    // nowhere else to point than the declaration.
    let line = (definition.start_line..=definition.end_line)
        .find(|line| idiom::normalize(&mask.outside_comments(*line)) == *only)
        .unwrap_or(definition.start_line);
    Some((line, only.to_string()))
}

/// Whether an empty body is the declaration itself rather than work deferred:
/// a Python method written inside a class that states a shape.
fn declares_a_type(lang: Lang, mask: &Mask, outline: &Outline, definition: &Definition) -> bool {
    if lang != Lang::Python {
        return false;
    }
    let Some(class) = hierarchy::enclosing_type(outline, definition) else {
        return false;
    };
    hierarchy::bases(lang, &mask.code(class.start_line))
        .iter()
        .any(|base| DECLARING_BASES.contains(&base.as_str()))
}

/// A stub, as it will be quoted back to the reader.
struct Stub {
    what: String,
    why: &'static str,
    /// Whether this is the shape another type could write out: the language's
    /// word for a body nobody wrote, or a raise that says as much. A marker and
    /// an empty body are neither, whatever they are written inside.
    signal: bool,
}

impl Stub {
    fn marker(word: &str) -> Stub {
        Stub {
            what: format!("a work marker reached production code: `{word}`."),
            why: "a marker is a note to the author, and it ships as behaviour nobody wrote.",
            signal: false,
        }
    }

    fn token(word: &str) -> Stub {
        Stub {
            what: format!("an unwritten body reached production code: `{word}`."),
            why: "the line compiles and fails at the moment a caller reaches it.",
            signal: true,
        }
    }

    fn announcement() -> Stub {
        Stub {
            what: "production code was added that raises to say it is not implemented.".to_string(),
            why: "the line compiles and fails at the moment a caller reaches it.",
            signal: true,
        }
    }

    fn empty_body(name: &str, statement: &str) -> Stub {
        Stub {
            what: format!("`{name}` was added with a body that does nothing: `{statement}`."),
            why: "a function that answers without working is a promise the caller cannot tell from a result.",
            signal: false,
        }
    }
}

/// What weeder says about one stub, once the tree has answered for it.
///
/// A signal in a base that nothing writes out is still the block it always was,
/// and the message says what was looked for, so a reader who knows better than
/// weeder can see which question it asked. A base an entry file states is the
/// one case weeder will not settle: the package hands the contract out, and the
/// type that fills it in may be in a repository this run cannot read. That is a
/// warning for the person at the pull request rather than a stopped commit.
fn finding(path: &str, found: &Found, tree: &Hierarchy) -> Detection {
    let exported = found
        .contract
        .as_ref()
        .and_then(|contract| tree.exported_through(&contract.base));
    let (stamp, why, next) = match (&found.contract, exported) {
        (Some(contract), Some(entry)) => (
            Stamp::Override(Level::Warn),
            format!(
                "`{}` is declared on `{}` and nothing in the repository overrides it, and `{entry}` states `{}` to whatever reads this package, so the override may live in another repository.",
                contract.method, contract.base, contract.base
            ),
            "write it here, or name the implementation that fills it in with a `Weeder-allow: S1` trailer.".to_string(),
        ),
        (Some(contract), None) => (
            Stamp::Rule,
            format!(
                "`{}` is declared on `{}` and nothing in the repository overrides it, so the failure is what every caller gets.",
                contract.method, contract.base
            ),
            TAKE_IT_BACK.to_string(),
        ),
        (None, _) => (
            Stamp::Rule,
            found.stub.why.to_string(),
            TAKE_IT_BACK.to_string(),
        ),
    };
    Detection {
        stamp,
        path: path.to_string(),
        region: Some(Region {
            start_line: found.line,
            end_line: found.line,
        }),
        message: Message {
            what: found.stub.what.clone(),
            why,
            next,
        },
        fix: None,
    }
}

/// What a reader is asked to do about a stub weeder will not settle for them.
const TAKE_IT_BACK: &str =
    "finish the work, take the line back out of the change, or name what fills it in with a `Weeder-allow: S1` trailer.";
