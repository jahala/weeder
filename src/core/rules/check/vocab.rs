//! The vocabulary a test suite is written in.
//!
//! Four rules read a suite for the same kinds of word, so the words live in one
//! table. T2 counts what a file asserts. T4 reads the number that says how much
//! a test will put up with. T6 reads what an assertion says about a failure. M1
//! reads the word that puts a double where a unit was, and T11 the one that
//! makes a double a case can assert on. Each language spells all four in the
//! grammar of its own frameworks, and none of them is named here:
//! what is written down is the shape, a call, a member on a receiver, a
//! keyword, a macro, and the roots those shapes are built from.
//!
//! T9 asks the other question of the same table, whether an added case
//! asserts at all, and a case can do that without a line that asserts: Rust
//! declares a test that should panic, or one that returns a `Result` and hands
//! a failure back with `?`, and the table carries both.
//!
//! A repository's own helpers are the one part of the vocabulary no table can
//! carry, so `[tests] assertions` names them and [`Suite::counting`] reads a
//! call to one as an assertion in any language. The shell calls nothing: its
//! suites assert by running a command, so a bash line is read as the commands
//! it runs rather than as the calls it makes.
//!
//! A line is read through the syntax mask, so a word inside a string or a
//! comment says nothing. Names are read as paths, `self.assertEqual`,
//! `require.NoError`, `Error::Empty`, because the receiver is half of what a
//! name means, and an identifier is read as the words it is built from, so
//! `toBeCloseTo`, `from_millis` and `Abs` all answer for themselves.

use std::collections::BTreeSet;

use crate::core::classify::Lang;
use crate::core::read::TestUnit;
use crate::core::shell;
use crate::core::syntax::{is_identifier, Mask};

/// One name as a line writes it: the segments of a path spelled with `.` or
/// `::`, and what follows the last of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name<'a> {
    /// The path as it is written.
    pub text: &'a str,
    /// Its segments, in order.
    pub segments: Vec<&'a str>,
    /// Where the path begins in the line, in bytes.
    pub start: usize,
    /// Where it ends.
    pub end: usize,
    /// The nearest character after the name that is not a space.
    pub after: Option<char>,
}

impl<'a> Name<'a> {
    /// The last segment: the thing named, whatever it was reached through.
    #[must_use]
    pub fn last(&self) -> &'a str {
        self.segments.last().copied().unwrap_or_default()
    }

    /// What the last segment was reached through, where it was reached through
    /// anything.
    #[must_use]
    pub fn receiver(&self) -> Option<&'a str> {
        self.segments
            .len()
            .checked_sub(2)
            .and_then(|index| self.segments.get(index).copied())
    }

    /// Whether the name is called.
    #[must_use]
    pub fn is_called(&self) -> bool {
        self.after == Some('(')
    }

    /// Whether the name is a macro: Rust's way of calling something that is not
    /// a function.
    #[must_use]
    pub fn is_macro(&self) -> bool {
        self.after == Some('!')
    }

    /// Whether the name names a type or a value the language capitalises, an
    /// error kind, a sentinel, a class. Every language weeder reads writes those
    /// with a capital and its ordinary bindings without one.
    #[must_use]
    pub fn is_capitalised(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.starts_with(char::is_uppercase))
    }
}

/// The names a line writes, each one whole.
#[must_use]
pub fn names(line: &str) -> Vec<Name<'_>> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (index, character) in line.char_indices() {
        if is_identifier(character) {
            start.get_or_insert(index);
        } else if let Some(from) = start.take() {
            spans.push((from, index));
        }
    }
    if let Some(from) = start {
        spans.push((from, line.len()));
    }

    let mut found: Vec<Name<'_>> = Vec::new();
    for (from, to) in spans {
        let joined = found
            .last()
            .is_some_and(|last| matches!(&line[last.end..from], "." | "::"));
        match found.last_mut() {
            Some(last) if joined => {
                last.segments.push(&line[from..to]);
                last.end = to;
                last.text = &line[last.start..to];
            }
            _ => found.push(Name {
                text: &line[from..to],
                segments: vec![&line[from..to]],
                start: from,
                end: to,
                after: None,
            }),
        }
    }
    for name in &mut found {
        name.after = line[name.end..].chars().find(|c| !c.is_whitespace());
    }
    found
}

/// An identifier as the words it is built from, lower-cased: `toBeCloseTo` is
/// four words, `from_millis` is two, `Abs` is one. This is how a rule asks what
/// a name is about without knowing which framework wrote it.
#[must_use]
pub fn parts(identifier: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for character in identifier.chars() {
        let breaks = character == '_' || character == '$' || character == '-';
        let hump = character.is_uppercase() && !current.is_empty();
        if breaks || hump {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            if breaks {
                continue;
            }
        }
        current.extend(character.to_lowercase());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Whether a name begins with a root and carries on in a framework's own
/// casing: `assert`, `assert_eq`, `assertEqual` all begin with `assert`, and
/// `assertion` does not begin with anything.
#[must_use]
pub fn begins_with_root(name: &str, root: &str) -> bool {
    let Some(rest) = name.strip_prefix(root) else {
        return false;
    };
    rest.is_empty()
        || rest.starts_with('_')
        || rest.starts_with(char::is_uppercase)
        || rest.starts_with(char::is_numeric)
}

/// Whether one of a name's words is a root, allowing the plural a package name
/// is often written in: `errors` answers for `error`.
#[must_use]
pub fn holds_word(identifier: &str, roots: &[&str]) -> bool {
    parts(identifier)
        .iter()
        .any(|part| roots.iter().any(|root| same_word(part, root)))
}

/// What one language's test frameworks call an assertion.
struct Assertions {
    /// A statement that asserts by keyword alone.
    keyword: Option<&'static str>,
    /// Names that assert when they are called, whole or as a root.
    roots: &'static [&'static str],
    /// Receivers whose every called member asserts: the libraries a suite
    /// reaches for when it does not write the assertion itself.
    receivers: &'static [&'static str],
    /// Members that report a failure on the handle the runner passes a test.
    handle_members: &'static [&'static str],
    /// Members that open a test inside a test on that same handle. Only a
    /// language whose runner declares one test and runs many under it has one.
    handle_subtests: &'static [&'static str],
    /// Commands that assert by running, in a language whose calls are
    /// commands: the name at the front of a command, read the way the shell
    /// reads one. A language with any of these is read for commands rather
    /// than for calls.
    commands: &'static [&'static str],
    /// Attributes a case can be declared under that make it pass only by
    /// failing, matched by the last segment of the path they are written with.
    expecting: &'static [&'static str],
    /// How a case can fail by what it returns, in a language where a test may
    /// return a failure to its runner.
    returned: Option<Returned>,
}

/// How a test hands a failure back to its runner instead of asserting: it is
/// declared to return a type that carries one, and an operator inside it
/// returns early with whatever failure it meets.
#[derive(Clone, Copy)]
struct Returned {
    /// What a declaration writes before the type it returns.
    arrow: &'static str,
    /// The type, by the last segment of its path, that carries a failure.
    carrier: &'static str,
    /// The operator that returns early with a failure.
    operator: char,
}

const NOTHING: Assertions = Assertions {
    keyword: None,
    roots: &[],
    receivers: &[],
    handle_members: &[],
    handle_subtests: &[],
    commands: &[],
    expecting: &[],
    returned: None,
};

fn assertions(lang: Lang) -> Assertions {
    match lang {
        // `expect(…)` and its matcher, `assert(…)`, and the assertion libraries
        // that hang everything off one object.
        Lang::TypeScript | Lang::JavaScript => Assertions {
            roots: &["expect", "assert"],
            receivers: &["assert", "chai", "should"],
            ..NOTHING
        },
        // `assert` is a statement; the unittest family writes methods that
        // begin with the same word, and a raise is asserted by name.
        Lang::Python => Assertions {
            keyword: Some("assert"),
            roots: &["assert", "raises", "fail"],
            ..NOTHING
        },
        // The macros, and the debug forms that compile away in release. A
        // test may also be declared to panic, or to return a `Result`, and
        // then a failure it meets is the runner's to report.
        Lang::Rust => Assertions {
            roots: &["assert", "debug_assert"],
            expecting: &["should_panic"],
            returned: Some(Returned {
                arrow: "->",
                carrier: "Result",
                operator: '?',
            }),
            ..NOTHING
        },
        // The standard library asserts nothing: a test fails by telling the
        // handle it was given. The table-driven suites reach for a library on
        // top of that, and both are read here.
        Lang::Go => Assertions {
            receivers: &["require", "assert"],
            handle_members: &["Error", "Fatal", "Fail"],
            handle_subtests: &["Run"],
            ..NOTHING
        },
        // A shell suite fails on a command that fails, and the commands that
        // exist to fail are the shell's own tests. The helper libraries bats
        // suites load assert and refute by name.
        Lang::Bash => Assertions {
            roots: &["assert", "refute"],
            commands: &["[", "[[", "test"],
            ..NOTHING
        },
        Lang::Other => NOTHING,
    }
}

/// One file's vocabulary: its language's table, what the file itself calls the
/// handle its runner passes in, and the helpers the repository says its suites
/// assert with, none of which a table can know in advance.
pub struct Suite {
    lang: Lang,
    handles: BTreeSet<String>,
    helpers: Vec<String>,
}

impl Suite {
    /// The vocabulary a file is read with. Only a language whose tests report a
    /// failure to a handle has one to look for, so only those files are read
    /// for it.
    #[must_use]
    pub fn of(lang: Lang, mask: &Mask) -> Suite {
        let table = assertions(lang);
        let handles = if table.handle_members.is_empty() && table.handle_subtests.is_empty() {
            BTreeSet::new()
        } else {
            handles(mask)
        };
        Suite {
            lang,
            handles,
            helpers: Vec::new(),
        }
    }

    /// The same vocabulary, with the repository's own helpers read as
    /// assertions: `[tests] assertions` in `weeder.toml`. A helper is matched
    /// by the whole name it is called by, so naming `t` makes `t(…)` a claim
    /// and leaves `test(…)` and `t.Run(…)` what they were.
    #[must_use]
    pub fn counting(mut self, helpers: &[String]) -> Suite {
        self.helpers = helpers.to_vec();
        self
    }

    /// Whether the 1-based line opens a test inside a test: the member a runner
    /// gives its handle for exactly that, called on the handle this file named.
    #[must_use]
    pub fn opens_subtest(&self, mask: &Mask, line: u32) -> bool {
        self.subtest_at(mask, line).is_some()
    }

    /// Where on the 1-based line the call that opens a test inside a test ends
    /// its name, as a column counted in characters: the call's arguments, and
    /// the body of the subtest with them, open after it.
    #[must_use]
    pub fn subtest_at(&self, mask: &Mask, line: u32) -> Option<usize> {
        let table = assertions(self.lang);
        let code = mask.code(line);
        names(&code)
            .iter()
            .find(|name| {
                name.is_called()
                    && name
                        .receiver()
                        .is_some_and(|receiver| self.handles.contains(receiver))
                    && table.handle_subtests.contains(&name.last())
            })
            .map(|name| code[..name.end].chars().count())
    }

    /// Whether a case is declared to pass only by failing: written under an
    /// attribute its runner reads as an expected failure. Such a case asserts
    /// through how it ends rather than through a line of its body.
    #[must_use]
    pub fn expects_failure(&self, case: &TestUnit) -> bool {
        let table = assertions(self.lang);
        case.attributes.iter().any(|attribute| {
            let path = attribute.split('(').next().unwrap_or(attribute);
            names(path)
                .first()
                .is_some_and(|name| table.expecting.contains(&name.last()))
        })
    }

    /// Whether a declaration says its case fails by what it returns: the type
    /// it names after the language's return arrow is one that carries a
    /// failure, whatever path it is reached through.
    #[must_use]
    pub fn returns_failure(&self, signature: &str) -> bool {
        let Some(returned) = assertions(self.lang).returned else {
            return false;
        };
        signature
            .split_once(returned.arrow)
            .is_some_and(|(_, after)| {
                names(after)
                    .first()
                    .is_some_and(|name| name.last() == returned.carrier)
            })
    }

    /// How many failures the 1-based line hands back to the runner, where the
    /// case it sits in returns them: the operator for that, written as code.
    /// The same character in front of a name is a bound on a type, `?Sized`,
    /// and hands nothing back.
    #[must_use]
    pub fn hands_back(&self, mask: &Mask, line: u32) -> usize {
        let Some(returned) = assertions(self.lang).returned else {
            return 0;
        };
        let code = mask.code(line);
        code.char_indices()
            .filter(|(at, character)| {
                *character == returned.operator
                    && !code[at + character.len_utf8()..]
                        .chars()
                        .next()
                        .is_some_and(is_identifier)
            })
            .count()
    }

    /// How many assertions the whole file makes.
    #[must_use]
    pub fn assertion_count(&self, mask: &Mask) -> usize {
        (1..=mask.line_count())
            .map(|line| self.assertions(mask, line))
            .sum()
    }

    /// How many assertions the 1-based line makes.
    #[must_use]
    pub fn assertions(&self, mask: &Mask, line: u32) -> usize {
        let code = mask.code(line);
        if !assertions(self.lang).commands.is_empty() {
            return self.asserting_commands(&code);
        }
        names(&code)
            .iter()
            .filter(|name| self.asserts(name))
            .count()
    }

    /// The names a line of code asserts with, in the order the line writes
    /// them.
    #[must_use]
    pub fn asserting<'c>(&self, code: &'c str) -> Vec<Name<'c>> {
        names(code)
            .into_iter()
            .filter(|name| self.asserts(name))
            .collect()
    }

    /// How many commands a line of a shell suite runs that assert: a test the
    /// shell runs, `[`, `[[` or `test`, a helper whose name carries a root, or
    /// one the repository named. A test that an `if`, an `elif`, a `while` or
    /// an `until` asks is the condition of that statement, and claims nothing.
    fn asserting_commands(&self, code: &str) -> usize {
        let table = assertions(self.lang);
        shell::commands(code)
            .iter()
            .filter(|command| {
                let (command, condition) = shell::simple(command);
                let Some(name) = command.first() else {
                    return false;
                };
                !condition
                    && (table.commands.contains(&name.as_str())
                        || table.roots.iter().any(|root| begins_with_root(name, root))
                        || self.helpers.contains(name))
            })
            .count()
    }

    /// Whether one name is an assertion.
    fn asserts(&self, name: &Name<'_>) -> bool {
        let table = assertions(self.lang);
        let last = name.last();
        if table.keyword == Some(last) && name.segments.len() == 1 {
            return true;
        }
        if !name.is_called() && !name.is_macro() {
            return false;
        }
        if self.helpers.iter().any(|helper| helper == name.text) {
            return true;
        }
        if table.roots.iter().any(|root| begins_with_root(last, root)) {
            return true;
        }
        let Some(receiver) = name.receiver() else {
            return false;
        };
        if table.receivers.contains(&receiver) {
            return true;
        }
        // A member of the handle carries the root and the runner's own suffix
        // on it, the formatting variant, the one that stops the test there ,
        // so the root is enough, and only on the handle itself.
        self.handles.contains(receiver)
            && table
                .handle_members
                .iter()
                .any(|member| last.starts_with(member))
    }
}

/// What a file calls the handle its runner passes a test. Go names the type on
/// the parameter, so the name in front of it is the handle however the author
/// spelled it, and a file that names none has none.
fn handles(mask: &Mask) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in 1..=mask.line_count() {
        let code = mask.code(line);
        let named = names(&code);
        for (index, name) in named.iter().enumerate() {
            if name.segments.first() != Some(&"testing") {
                continue;
            }
            let Some(before) = index.checked_sub(1).and_then(|at| named.get(at)) else {
                continue;
            };
            // Only whitespace and the pointer star may sit between the
            // parameter and the type it is declared with.
            if code[before.end..name.start]
                .chars()
                .all(|character| character == '*' || character.is_whitespace())
            {
                found.insert(before.text.to_string());
            }
        }
    }
    found
}

/// Which way a number widens what a test will accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sense {
    /// The number is the slack itself, or the time a test will wait: larger
    /// accepts more.
    Magnitude,
    /// The number is how many digits have to match: smaller accepts more.
    Precision,
    /// The number is a nearness the framework spells either way. Which one it
    /// is, is read off the number: a slack is written with a fraction or an
    /// exponent, and a count of digits is a whole number.
    Nearness,
}

/// The words that say a number is a slack, how far apart two values may be, or
/// how long a test will wait, and which way that number widens.
const SLACK_WORDS: &[(&str, Sense)] = &[
    ("tolerance", Sense::Magnitude),
    ("epsilon", Sense::Magnitude),
    ("eps", Sense::Magnitude),
    ("delta", Sense::Magnitude),
    ("margin", Sense::Magnitude),
    ("abs", Sense::Magnitude),
    ("rel", Sense::Magnitude),
    ("places", Sense::Precision),
    ("digits", Sense::Precision),
    ("ndigits", Sense::Precision),
    ("precision", Sense::Precision),
    ("decimals", Sense::Precision),
    ("approx", Sense::Nearness),
    ("almost", Sense::Nearness),
    ("close", Sense::Nearness),
    ("near", Sense::Nearness),
    ("within", Sense::Nearness),
    ("timeout", Sense::Magnitude),
    ("deadline", Sense::Magnitude),
    ("sleep", Sense::Magnitude),
    ("wait", Sense::Magnitude),
    ("delay", Sense::Magnitude),
    ("interval", Sense::Magnitude),
    ("duration", Sense::Magnitude),
    ("retries", Sense::Magnitude),
    ("nanos", Sense::Magnitude),
    ("micros", Sense::Magnitude),
    ("millis", Sense::Magnitude),
    ("milliseconds", Sense::Magnitude),
    ("secs", Sense::Magnitude),
    ("seconds", Sense::Magnitude),
    ("minutes", Sense::Magnitude),
];

/// What a name says about a number written beside it, or nothing where the name
/// says nothing about slack at all. A unit is matched in the singular or the
/// plural, because a framework writes `Millisecond` where a flag writes `millis`.
#[must_use]
pub fn slack(identifier: &str) -> Option<Sense> {
    let words = parts(identifier);
    SLACK_WORDS
        .iter()
        .find(|(word, _)| words.iter().any(|part| same_word(part, word)))
        .map(|(_, sense)| *sense)
}

/// Whether two words are the same word, one of them possibly a plural.
fn same_word(part: &str, word: &str) -> bool {
    part == word || part.strip_suffix('s') == Some(word) || word.strip_suffix('s') == Some(part)
}

/// The words that say a line is about a failure rather than about a value.
const ERROR_WORDS: &[&str] = &[
    "error",
    "err",
    "throw",
    "raise",
    "panic",
    "exception",
    "fail",
    "reject",
];

/// The names a language gives the failure that could be any failure at all. An
/// assertion that names one of these has named nothing.
const CATCH_ALL_ERRORS: &[&str] = &[
    "Error",
    "Exception",
    "BaseException",
    "Throwable",
    "AnyError",
];

/// Whether a line of code says anything about a failure.
#[must_use]
pub fn mentions_failure(code: &str) -> bool {
    names(code).iter().any(|name| {
        name.segments
            .iter()
            .any(|segment| holds_word(segment, ERROR_WORDS))
    })
}

/// What a line says about the failure it asserts on: the error kinds it names
/// and the messages it matches. An assertion that carries none of these accepts
/// any failure at all.
#[must_use]
pub fn failure_qualifiers(mask: &Mask, line: u32) -> BTreeSet<String> {
    let mut qualifiers = BTreeSet::new();
    let code = mask.code(line);
    for name in names(&code) {
        // A name that is called is the assertion's own machinery; what it is
        // called with is the claim, and that is what a qualifier is.
        if name.is_called() || name.is_macro() {
            continue;
        }
        if name.is_capitalised() && !CATCH_ALL_ERRORS.contains(&name.text) {
            qualifiers.insert(name.text.to_string());
        }
    }
    for literal in mask.literal_runs(line) {
        let inside = literal.text.trim_matches(['"', '\'', '`']).trim();
        if !inside.is_empty() {
            qualifiers.insert(inside.to_string());
        }
    }
    qualifiers
}

/// The words that say a name puts a double where a unit was.
const MOCK_WORDS: &[&str] = &["mock", "patch", "stub", "fake", "double", "spy"];

/// Whether a name replaces something with a double.
#[must_use]
pub fn is_mock(identifier: &str) -> bool {
    holds_word(identifier, MOCK_WORDS)
}

/// The member the script frameworks make a mock function with: a function
/// made by a member called `fn`, reached through the framework's object and
/// called. It is read as that shape, so no framework is named.
const SCRIPT_DOUBLE_MEMBER: &str = "fn";

/// Whether a name, called or built, makes a double: the thing called, or a
/// type it is reached through, says so, or, in the script languages, it is the
/// member that makes a mock function. A value merely called `fake` makes
/// nothing when a method of it is called: `fake.Count()` reads a double rather
/// than making one.
#[must_use]
pub fn makes_a_double(lang: Lang, name: &Name<'_>) -> bool {
    let last = name.segments.len().saturating_sub(1);
    name.segments.iter().enumerate().any(|(at, segment)| {
        (at == last || segment.starts_with(char::is_uppercase)) && is_mock(segment)
    }) || (matches!(lang, Lang::TypeScript | Lang::JavaScript)
        && name.is_called()
        && name.receiver().is_some()
        && name.last() == SCRIPT_DOUBLE_MEMBER)
}

/// What a doubled name stands in for: the words left over once the word that
/// made it a double is taken out. `MockFormatter` and `NewMockFormatter` both
/// stand in for `Formatter`; a name that is only the word itself stands in for
/// nothing.
#[must_use]
pub fn doubled_symbol(identifier: &str) -> Option<String> {
    let words = parts(identifier);
    let at = words
        .iter()
        .position(|word| MOCK_WORDS.contains(&word.as_str()))?;
    let rest = &words[at + 1..];
    if rest.is_empty() {
        return None;
    }
    // The name is put back together the way the identifier spelled it, which is
    // the way the declaration it stands in for is spelled too.
    let capitalised = identifier.chars().next().is_some_and(char::is_uppercase)
        || identifier.contains(char::is_uppercase);
    Some(if capitalised {
        rest.iter().map(|word| capitalise(word)).collect()
    } else {
        rest.join("_")
    })
}

fn capitalise(word: &str) -> String {
    let mut characters = word.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

/// One number a line writes, and where it wrote it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number {
    pub start: usize,
    pub end: usize,
    pub value: f64,
}

/// The numbers a line of code writes, in the order it writes them.
///
/// A run of digits that a name runs into is part of the name and not a number.
/// A literal a suffix runs into, a radix, a unit, a type, is a number weeder
/// cannot compare against another one, so it is left where it is rather than
/// read wrongly.
#[must_use]
pub fn numbers(code: &str) -> Vec<Number> {
    let characters: Vec<(usize, char)> = code.char_indices().collect();
    let at = |index: usize| characters.get(index).map(|(_, character)| *character);
    let offset = |index: usize| {
        characters
            .get(index)
            .map_or(code.len(), |(offset, _)| *offset)
    };
    let digit = |index: usize| at(index).is_some_and(|character| character.is_ascii_digit());

    let mut found = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if !digit(index) {
            index += 1;
            continue;
        }
        let preceded = index
            .checked_sub(1)
            .and_then(at)
            .is_some_and(|character| is_identifier(character) || character == '.');
        if preceded {
            index += 1;
            continue;
        }

        let start = offset(index);
        let mut end = index;
        while digit(end) || (at(end) == Some('_') && digit(end + 1)) {
            end += 1;
        }
        if at(end) == Some('.') && digit(end + 1) {
            end += 1;
            while digit(end) {
                end += 1;
            }
        }
        if matches!(at(end), Some('e' | 'E')) {
            let exponent = usize::from(matches!(at(end + 1), Some('+' | '-')));
            if digit(end + 1 + exponent) {
                end += 1 + exponent;
                while digit(end) {
                    end += 1;
                }
            }
        }

        let stop = offset(end);
        if !at(end).is_some_and(is_identifier) {
            if let Ok(value) = code[start..stop].replace('_', "").parse::<f64>() {
                found.push(Number {
                    start,
                    end: stop,
                    value,
                });
            }
        }
        index = end.max(index + 1);
    }
    found
}

/// A line of code with its numbers taken out, so two lines that differ only in
/// what they will accept compare equal.
#[must_use]
pub fn skeleton(code: &str) -> String {
    let mut skeleton = String::new();
    let mut cut = 0;
    for number in numbers(code) {
        skeleton.push_str(&code[cut..number.start]);
        skeleton.push('#');
        cut = number.end;
    }
    skeleton.push_str(&code[cut..]);
    skeleton.split_whitespace().collect::<Vec<&str>>().join(" ")
}
