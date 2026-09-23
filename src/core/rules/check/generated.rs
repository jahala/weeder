//! The test cases a file generates rather than writes out.
//!
//! A suite that grows stops declaring its cases one at a time. Three `it` calls
//! become a table and a `for` around one call; three Go tests become one test
//! with a `t.Run` in a range; three Python functions become one function and a
//! list the decorator is handed; three Rust tests become a macro invoked once
//! per line. Nothing is covered less afterwards, and a reader that counts
//! declarations counts the difference as a deletion.
//!
//! So weeder counts the table instead. A declaration written inside a loop stands
//! for one case per entry in what the loop runs over; a test that runs subtests
//! over a table stands for one per entry too; a declaration handed a table by
//! the decorator above it stands for one per entry, as does a case handed its
//! table through `each` or one row at a time by attributes; and a macro whose
//! body carries a test attribute gives one case for each entry it is invoked
//! with.
//! Where the entries cannot be counted, a generated declaration stands for the
//! one case that is written, which is what the file says on its face.
//!
//! Some of those declarations the reader never reports at all: a title that is
//! a variable rather than a literal, a function a decorator stands in front of.
//! Those are read off the lines that declare them, so a case counts whether or
//! not the substrate could name it.
//!
//! What a table does to a declaration it does to every line under it, so the
//! same tables answer how many times a line runs: an assertion inside a loop
//! over three cases is made three times, and one inside a case the table hands
//! three rows is too. [`Runs`] is that answer, and it says so where the entries
//! cannot be counted rather than guessing at them.
//!
//! Everything is read through the syntax mask, so a comma inside a string never
//! ends an entry and a brace inside a comment never opens a table.

use crate::core::change::Side;
use crate::core::classify::Lang;
use crate::core::read::TestUnit;
use crate::core::rules::check::idiom::{self, Block};
use crate::core::rules::check::vocab::{names, Suite};
use crate::core::syntax::{is_identifier, words, Mask};

/// The keyword every language weeder reads opens a loop over a collection with.
const LOOP_KEYWORD: &str = "for";

/// The words a language writes between a loop's binding and what it runs over.
const RUNS_OVER: &[&str] = &["of", "in", "range"];

/// The names that declare a case where the language spells one as a call. The
/// words that group cases rather than declare one are not here: a suite that
/// holds nothing is not a case.
const CASE_CALLS: &[&str] = &["it", "test", "bench"];

/// The name that groups cases where the language spells a suite as a call. A
/// table handed to a suite runs everything the suite holds once per row.
const SUITE_CALL: &str = "describe";

/// The member a call-declared case or suite is handed a table through.
const EACH: &str = "each";

/// The methods that walk a collection without changing how many entries it
/// has. A loop over anything else a call hands back runs a number of times
/// weeder cannot read off the file.
const WALKS: &[&str] = &[
    "iter",
    "into_iter",
    "iter_mut",
    "copied",
    "cloned",
    "enumerate",
    "items",
    "keys",
    "values",
    "entries",
];

/// What Python writes in front of a function it is declaring.
const PYTHON_DECLARATION: &str = "def";

/// What Python writes in front of a declaration or loop that runs as a
/// coroutine.
const PYTHON_ASYNC: &str = "async";

/// What a Python test function's name begins with, which is how its runners
/// collect one.
const PYTHON_CASE: &str = "test";

/// The character a decorator opens with.
const DECORATOR: char = '@';

/// What Rust writes in front of a function it is declaring.
const RUST_DECLARATION: &str = "fn";

/// What an attribute opens with.
const ATTRIBUTE: &str = "#[";

/// The attribute that hands a Rust test one row of its table.
const CASE_ATTRIBUTE: &str = "case";

/// What Rust writes in front of a macro it is declaring.
const MACRO_KEYWORD: &str = "macro_rules";

/// The attribute that makes a function a test, whichever runtime wraps it.
const TEST_ATTRIBUTE: &str = "test";

/// How many names weeder follows from one to the next before it stops. A table
/// is given a name once, and a name given another name is rare; the bound is
/// what keeps a file that assigns names to each other in a ring from being
/// followed round it forever.
const HOPS: usize = 4;

/// A place in a file: a 1-based line, and a column counted in characters.
type At = (u32, usize);

/// One loop, the lines it runs over, and how many entries what it runs over
/// holds, where the file writes them down.
struct Loop {
    at: u32,
    body: Block,
    entries: Option<usize>,
}

/// How many cases a file holds, the generated ones counted by the tables that
/// generate them.
#[must_use]
pub fn case_count(side: &Side) -> usize {
    let lang = side.lang();
    let mask = side.mask();
    let loops = loops(lang, mask);
    let suite = Suite::of(lang, mask);
    let declared: Vec<&TestUnit> = side.tests.cases().collect();
    let handed = match lang {
        Lang::TypeScript | Lang::JavaScript => each_tables(lang, mask),
        _ => Vec::new(),
    };

    let mut total = 0;
    for case in &declared {
        total += subtests(mask, &suite, &loops, case)
            .unwrap_or_else(|| stands_for(mask, &loops, &handed, case.start_line));
    }
    for line in unseen(lang, mask, &declared) {
        total += stands_for(mask, &loops, &handed, line);
    }
    total + macro_generated(lang, mask) + row_generated(lang, mask, &declared)
}

/// How many times a line runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Times {
    /// Once for each entry of every table that drives it, and once where
    /// nothing does.
    Counted(usize),
    /// Under a loop or a table whose entries the file does not write down, so
    /// the line runs at least once and weeder cannot say how many times more.
    Uncounted,
}

/// The tables that drive the lines of one file: every loop, and every
/// declaration handed rows to run once each.
pub struct Runs {
    drivers: Vec<Driver>,
}

/// One table, the lines it runs, and how many entries it holds where the file
/// writes them down.
struct Driver {
    first: u32,
    last: u32,
    entries: Option<usize>,
}

impl Runs {
    /// The tables of one file, read in the grammar of the language that wrote it.
    #[must_use]
    pub fn of(lang: Lang, mask: &Mask) -> Runs {
        let mut drivers: Vec<Driver> = loops(lang, mask)
            .into_iter()
            .map(|opened| Driver {
                first: opened.at,
                last: opened.body.last,
                entries: opened.entries,
            })
            .collect();
        match lang {
            Lang::Python => drivers.extend(decorated_functions(mask)),
            Lang::TypeScript | Lang::JavaScript => drivers.extend(each_tables(lang, mask)),
            Lang::Rust => {
                drivers.extend(case_attributes(mask));
                drivers.extend(macro_bodies(mask));
            }
            Lang::Go | Lang::Other => {}
        }
        Runs { drivers }
    }

    /// How many times the 1-based line runs: the product of the entries of
    /// every table around it.
    #[must_use]
    pub fn times(&self, line: u32) -> Times {
        let mut total: usize = 1;
        for driver in &self.drivers {
            if line < driver.first || driver.last < line {
                continue;
            }
            match driver.entries {
                Some(entries) => total = total.saturating_mul(entries),
                None => return Times::Uncounted,
            }
        }
        Times::Counted(total)
    }
}

/// How many cases one declaration stands for: the entries of the table driving
/// it, or the one case that is written.
fn stands_for(mask: &Mask, loops: &[Loop], handed: &[Driver], line: u32) -> usize {
    around(loops, line)
        .or_else(|| decorated(mask, line))
        .or_else(|| {
            handed
                .iter()
                .find(|table| table.first == line)
                .and_then(|table| table.entries)
        })
        .unwrap_or(1)
}

/// The entries of the table the innermost loop around a declaration runs over.
fn around(loops: &[Loop], line: u32) -> Option<usize> {
    loops
        .iter()
        .filter(|opened| opened.at < line && line <= opened.body.last)
        .max_by_key(|opened| opened.at)
        .and_then(|opened| opened.entries)
}

/// The entries of the table a test runs subtests over: a loop written inside
/// the case, whose body opens a test inside the test.
fn subtests(mask: &Mask, suite: &Suite, loops: &[Loop], case: &TestUnit) -> Option<usize> {
    loops
        .iter()
        .filter(|opened| opened.at > case.start_line && opened.body.last <= case.end_line)
        .find(|opened| (opened.at..=opened.body.last).any(|line| suite.opens_subtest(mask, line)))
        .and_then(|opened| opened.entries)
}

/// The entries of the table a decorator directly above a declaration hands it:
/// a table written in the decorator, or one it names that the file assigns.
fn decorated(mask: &Mask, line: u32) -> Option<usize> {
    let opened = *annotations_above(mask, line, &DECORATOR.to_string()).first()?;
    (opened..line)
        .find_map(|at| {
            let column = mask
                .code(at)
                .chars()
                .position(|character| character == '[')?;
            entries_at(mask, at, column)
        })
        .or_else(|| {
            (opened..line).find_map(|at| {
                let code = mask.code(at);
                words(&code)
                    .into_iter()
                    .filter(|word| !word.is_member() && !word.is_called())
                    .filter(|word| word.after != Some('=') && word.after != Some('.'))
                    .find_map(|word| named_entries(Lang::Python, mask, word.text, opened, HOPS))
            })
        })
}

/// The lines the annotations stacked directly above a declaration open on,
/// nearest first. The search walks up from the declaration, closing brackets
/// as it goes, so an annotation written across several lines is followed to
/// the line that opened it, and an ordinary statement above ends the search at
/// once. A comment between two annotations is passed over.
fn annotations_above(mask: &Mask, line: u32, marker: &str) -> Vec<u32> {
    let mut depth = 0_i32;
    let mut found = Vec::new();
    for above in (1..line).rev() {
        let code = mask.code(above);
        for character in code.chars() {
            depth += closing(character) - opening(character);
        }
        if depth > 0 {
            continue;
        }
        if code.trim().is_empty() && !mask.comments(above).trim().is_empty() {
            continue;
        }
        if !code.trim_start().starts_with(marker) {
            break;
        }
        found.push(above);
    }
    found
}

/// Every loop the file opens, with the lines each one runs over.
fn loops(lang: Lang, mask: &Mask) -> Vec<Loop> {
    let mut found = Vec::new();
    for at in 1..=mask.line_count() {
        let code = mask.code(at);
        let Some(keyword) = words(&code)
            .into_iter()
            .find(|word| word.text == LOOP_KEYWORD && !word.is_member())
        else {
            continue;
        };
        if !opens_statement(lang, &code[..keyword.start]) {
            continue;
        }
        let after = code[..keyword.end].chars().count();
        let over = words(&code)
            .into_iter()
            .filter(|word| RUNS_OVER.contains(&word.text) && !word.is_member())
            .map(|word| code[..word.end].chars().count())
            .rfind(|end| *end > after);
        let (entries, end) = match over {
            Some(from) => runs_over(lang, mask, (at, from), HOPS),
            None => (None, (at, after)),
        };
        let body = match lang {
            // An indented language opens its body at the colon closing the header.
            Lang::Python => idiom::block(lang, mask, at, after),
            _ => body_after(lang, mask, end).map(|last| Block { first: at, last }),
        };
        if let Some(body) = body {
            found.push(Loop { at, body, entries });
        }
    }
    found
}

/// Whether what a line writes in front of a keyword leaves the keyword opening
/// a statement: nothing at all, a label, or the end of a statement on the same
/// line. A `for` anywhere else is a comprehension in Python and an `impl` or a
/// bound in Rust, and runs no statement of its own.
fn opens_statement(lang: Lang, before: &str) -> bool {
    let before = before.trim_end();
    if lang == Lang::Python {
        return before.trim_start().is_empty() || before.trim_start() == PYTHON_ASYNC;
    }
    before.is_empty() || before.ends_with([':', ';', '{', '}'])
}

/// What an expression runs over, read from where it starts: how many entries
/// it holds where the file writes them down, and where the expression ends.
fn runs_over(lang: Lang, mask: &Mask, from: At, hops: usize) -> (Option<usize>, At) {
    let mut from = from;
    loop {
        let Some((at, first)) = next_code(mask, from) else {
            return (None, from);
        };
        return match first {
            // A borrow or a dereference runs over what it reaches.
            '&' | '*' => {
                from = step(at);
                continue;
            }
            // Go writes no bracket without a type, so one opens a composite.
            '[' if lang == Lang::Go => composite(mask, at),
            '(' | '[' | '{' => group(mask, at),
            character if is_identifier(character) => chain(lang, mask, at, hops),
            _ => (None, at),
        };
    }
}

/// The entries of the bracketed group opening at a place, and where it ends.
fn group(mask: &Mask, at: At) -> (Option<usize>, At) {
    match closing_of(mask, at) {
        Some(close) => (entries_at(mask, at.0, at.1), step(close)),
        None => (None, at),
    }
}

/// The entries of a Go composite literal: a type, then the table in braces. A
/// struct or interface type brings braces of its own, which open no table.
fn composite(mask: &Mask, from: At) -> (Option<usize>, At) {
    let mut at = from;
    loop {
        let Some((place, character)) = next_code(mask, at) else {
            return (None, at);
        };
        match character {
            '[' => match closing_of(mask, place) {
                Some(close) => at = step(close),
                None => return (None, place),
            },
            '*' | '.' => at = step(place),
            '{' => return group(mask, place),
            character if is_identifier(character) => {
                let (word, end) = identifier(mask, place);
                at = (place.0, end);
                if word == "struct" || word == "interface" {
                    let Some((open, '{')) = next_code(mask, at) else {
                        return (None, at);
                    };
                    match closing_of(mask, open) {
                        Some(close) => at = step(close),
                        None => return (None, open),
                    }
                }
            }
            _ => return (None, place),
        }
    }
}

/// The entries of the table a chain of names reaches: the name it starts from,
/// followed through its fields, and walked by methods that keep its length. A
/// chain that starts with a call, or passes through any other, runs over
/// something only running it would count.
fn chain(lang: Lang, mask: &Mask, from: At, hops: usize) -> (Option<usize>, At) {
    let mut at = from;
    let mut name: Option<String> = None;
    let mut counted = true;
    let mut called = false;
    loop {
        let (word, end) = identifier(mask, at);
        let mut next = next_code(mask, (at.0, end));
        match next {
            // A macro hands its own brackets the table.
            Some((bang, '!')) if lang == Lang::Rust && name.is_none() => {
                return match next_code(mask, step(bang)) {
                    Some((open, '(' | '[' | '{')) => group(mask, open),
                    _ => (None, step(bang)),
                };
            }
            Some((_, '[')) if lang == Lang::Go && word == "map" && name.is_none() => {
                return composite(mask, at);
            }
            Some((open, '(')) => {
                if name.is_none() || !WALKS.contains(&word.as_str()) {
                    counted = false;
                }
                called = true;
                let Some(close) = closing_of(mask, open) else {
                    return (None, open);
                };
                next = next_code(mask, step(close));
            }
            _ if called => counted = false,
            _ => name = Some(word),
        }
        let joined = match next {
            Some((dot, '.')) => Some(step(dot)),
            Some((colon, ':')) if char_at(mask, step(colon)) == Some(':') => {
                Some(step(step(colon)))
            }
            _ => None,
        };
        let Some(onward) = joined.and_then(|place| next_code(mask, place)) else {
            let end = next.map_or((at.0, end), |(place, _)| place);
            let entries = match (counted, name) {
                (true, Some(name)) => named_entries(lang, mask, &name, from.0, hops),
                _ => None,
            };
            return (entries, end);
        };
        if !is_identifier(onward.1) {
            return (None, onward.0);
        }
        at = onward.0;
    }
}

/// The entries of the table a name was given, where the statement that assigns
/// it gives it one: the nearest above the line that reads it, or, for a
/// constant a file declares after the code that reads it, the first below.
fn named_entries(lang: Lang, mask: &Mask, name: &str, reader: u32, hops: usize) -> Option<usize> {
    let hops = hops.checked_sub(1)?;
    let assigned = |line: u32| assigned_at(&mask.code(line), name).map(|value| (line, value));
    let (line, value) = (1..reader)
        .rev()
        .find_map(assigned)
        .or_else(|| (reader.saturating_add(1)..=mask.line_count()).find_map(assigned))?;
    runs_over(lang, mask, (line, value), hops).0
}

/// Where the value begins on a line that assigns `name`: past the operator that
/// joins the two. A line that only mentions the name assigns nothing.
fn assigned_at(code: &str, name: &str) -> Option<usize> {
    let word = words(code)
        .into_iter()
        .find(|word| word.text == name && !word.is_member())?;
    let characters: Vec<char> = code.chars().collect();
    let mut depth = 0_i32;
    let mut column = code[..word.end].chars().count();
    while let Some(character) = characters.get(column).copied() {
        // A comparison joins nothing, nor does an arrow, and a type written
        // between the name and the operator brings brackets of its own.
        let compares = matches!(characters.get(column + 1), Some('=' | '>'))
            || matches!(
                column.checked_sub(1).and_then(|at| characters.get(at)),
                Some('=' | '!' | '<' | '>')
            );
        if character == '=' && depth == 0 && !compares {
            return Some(column + 1);
        }
        depth += opening(character) - closing(character);
        column += 1;
    }
    None
}

/// How many entries the bracketed group opening at a column holds: the things
/// separated by the commas written at the depth the bracket opened. A comma
/// closing the last entry ends it rather than opening another, and a group with
/// nothing in it holds nothing. A literal is an entry like any other, though the
/// code view blanks it.
fn entries_at(mask: &Mask, line: u32, column: usize) -> Option<usize> {
    if opening(mask.code(line).chars().nth(column)?) != 1 {
        return None;
    }
    let mut depth = 0_i32;
    let mut commas = 0;
    let mut written = false;
    let mut trailing = false;
    for at in line..=mask.line_count() {
        let start = if at == line { column } else { 0 };
        for character in substance(mask, at).into_iter().skip(start) {
            let opens = opening(character) == 1;
            let closes = closing(character) == 1;
            if closes && depth == 1 {
                return Some(if written {
                    commas + usize::from(!trailing)
                } else {
                    0
                });
            }
            depth += opening(character) - closing(character);
            if character == ',' && depth == 1 {
                commas += 1;
                trailing = true;
            } else if !character.is_whitespace() && !(opens && depth == 1) {
                written = true;
                trailing = false;
            }
        }
    }
    None
}

/// A line as the code view reads it, with each character of a literal standing
/// as a mark that is neither a bracket nor a comma: something written, whose
/// punctuation is not the program's.
fn substance(mask: &Mask, line: u32) -> Vec<char> {
    mask.code(line)
        .chars()
        .zip(mask.literals(line).chars())
        .map(
            |(code, literal)| {
                if literal.is_whitespace() {
                    code
                } else {
                    '"'
                }
            },
        )
        .collect()
}

/// The lines declaring a case the reader's shape does not carry: a title given
/// as a name rather than as a literal, a function a decorator stands in front
/// of. A declaration the reader did report is left to it.
fn unseen(lang: Lang, mask: &Mask, declared: &[&TestUnit]) -> Vec<u32> {
    (1..=mask.line_count())
        .filter(|line| !declared.iter().any(|case| case.start_line == *line))
        .filter(|line| declares_a_case(lang, &mask.code(*line)))
        .collect()
}

/// Whether a line declares a case, in the grammar of the language that wrote it.
/// A case handed a table through `each` is declared by the line that hands it.
fn declares_a_case(lang: Lang, code: &str) -> bool {
    let spoken = words(code);
    match lang {
        Lang::TypeScript | Lang::JavaScript => {
            spoken.iter().any(|word| {
                CASE_CALLS.contains(&word.text) && word.is_called() && !word.is_member()
            }) || names(code).iter().any(|name| {
                name.segments
                    .first()
                    .is_some_and(|first| CASE_CALLS.contains(first))
                    && name.segments.len() > 1
                    && name.last() == EACH
            })
        }
        Lang::Python => {
            spoken
                .first()
                .is_some_and(|word| word.text == PYTHON_DECLARATION)
                && spoken.get(1).is_some_and(|word| {
                    word.text == PYTHON_CASE || word.text.starts_with(&format!("{PYTHON_CASE}_"))
                })
        }
        Lang::Rust | Lang::Go | Lang::Other => false,
    }
}

/// The Python functions a decorator hands a table, each running its body once
/// per entry.
fn decorated_functions(mask: &Mask) -> Vec<Driver> {
    let mut found = Vec::new();
    for line in 1..=mask.line_count() {
        let code = mask.code(line);
        let spoken = words(&code);
        let declares = match spoken.first() {
            Some(word) if word.text == PYTHON_DECLARATION => true,
            Some(word) if word.text == PYTHON_ASYNC => spoken
                .get(1)
                .is_some_and(|word| word.text == PYTHON_DECLARATION),
            _ => false,
        };
        if !declares {
            continue;
        }
        let Some(entries) = decorated(mask, line) else {
            continue;
        };
        if let Some(body) = idiom::block(Lang::Python, mask, line, 0) {
            found.push(Driver {
                first: line,
                last: body.last,
                entries: Some(entries),
            });
        }
    }
    found
}

/// The cases and suites a table is handed to through `each`, each running what
/// it holds once per row. A table written as a tagged template is rows of text
/// weeder does not split, so its entries are not counted.
fn each_tables(lang: Lang, mask: &Mask) -> Vec<Driver> {
    let mut found = Vec::new();
    for line in 1..=mask.line_count() {
        let code = mask.code(line);
        for name in names(&code) {
            let declares = name
                .segments
                .first()
                .is_some_and(|first| CASE_CALLS.contains(first) || *first == SUITE_CALL);
            if name.segments.len() < 2 || !declares || name.last() != EACH {
                continue;
            }
            let after = (line, code[..name.end].chars().count());
            let (entries, end) = match next_code(mask, after) {
                Some((open, '(')) => match closing_of(mask, open) {
                    Some(close) => (runs_over(lang, mask, step(open), HOPS).0, step(close)),
                    None => continue,
                },
                _ => (None, after),
            };
            let last = match next_code(mask, end) {
                Some((open, '(')) => closing_of(mask, open).map_or(end.0, |close| close.0),
                _ => end.0,
            };
            found.push(Driver {
                first: line,
                last,
                entries,
            });
        }
    }
    found
}

/// The Rust functions handed their rows one attribute at a time, each running
/// its body once per row.
fn case_attributes(mask: &Mask) -> Vec<Driver> {
    let mut found = Vec::new();
    for line in 1..=mask.line_count() {
        let code = mask.code(line);
        let Some(keyword) = words(&code)
            .into_iter()
            .find(|word| word.text == RUST_DECLARATION && !word.is_member())
        else {
            continue;
        };
        let rows = annotations_above(mask, line, ATTRIBUTE)
            .into_iter()
            .filter(|above| is_case_attribute(&mask.code(*above)))
            .count();
        if rows == 0 {
            continue;
        }
        let after = (line, code[..keyword.end].chars().count());
        if let Some(last) = body_after(Lang::Rust, mask, after) {
            found.push(Driver {
                first: line,
                last,
                entries: Some(rows),
            });
        }
    }
    found
}

/// Whether a line opens the attribute that hands a test one row: `#[case]`,
/// `#[case(…)]`, or a row given a name of its own.
fn is_case_attribute(code: &str) -> bool {
    let Some(inside) = code.trim_start().strip_prefix(ATTRIBUTE) else {
        return false;
    };
    let path = inside.split(['(', ']']).next().unwrap_or(inside).trim();
    let segments: Vec<&str> = path.split("::").collect();
    segments.first() == Some(&CASE_ATTRIBUTE) || segments.last() == Some(&CASE_ATTRIBUTE)
}

/// The bodies of the macros that write tests, each running once per entry of
/// every invocation.
fn macro_bodies(mask: &Mask) -> Vec<Driver> {
    test_macros(mask)
        .into_iter()
        .map(|(name, body)| {
            let entries = invocations(mask, &name, &body).into_iter().try_fold(
                0_usize,
                |total, (line, column)| {
                    entries_at(mask, line, column).map(|entries| total.saturating_add(entries))
                },
            );
            Driver {
                first: body.first,
                last: body.last,
                entries,
            }
        })
        .collect()
}

/// How many cases the Rust functions handed their rows one attribute at a time
/// generate beyond what the reader already counted: one per row. The reader may
/// report such a function as the one case it is written as, or not at all.
fn row_generated(lang: Lang, mask: &Mask, declared: &[&TestUnit]) -> usize {
    if lang != Lang::Rust {
        return 0;
    }
    case_attributes(mask)
        .iter()
        .map(|table| {
            let rows = table.entries.unwrap_or(1);
            let counted = declared
                .iter()
                .any(|case| case.start_line <= table.first && table.first <= case.end_line);
            if counted {
                rows.saturating_sub(1)
            } else {
                rows
            }
        })
        .fold(0, usize::saturating_add)
}

/// How many cases the macros a file declares generate: one per entry of every
/// invocation of a macro whose body carries a test attribute.
fn macro_generated(lang: Lang, mask: &Mask) -> usize {
    if lang != Lang::Rust {
        return 0;
    }
    let mut total = 0;
    for (name, body) in test_macros(mask) {
        for (line, column) in invocations(mask, &name, &body) {
            total += entries_at(mask, line, column).unwrap_or(1);
        }
    }
    total
}

/// The macros a file declares that write tests, each with the lines its
/// declaration occupies.
fn test_macros(mask: &Mask) -> Vec<(String, Block)> {
    let mut found = Vec::new();
    for at in 1..=mask.line_count() {
        let code = mask.code(at);
        let spoken = words(&code);
        let Some(index) = spoken
            .iter()
            .position(|word| word.text == MACRO_KEYWORD && word.after == Some('!'))
        else {
            continue;
        };
        let Some(name) = spoken.get(index + 1) else {
            continue;
        };
        let after = code[..name.end].chars().count();
        let Some(body) = idiom::block(Lang::Rust, mask, at, after) else {
            continue;
        };
        if (body.first..=body.last).any(|line| is_test_attribute(&mask.code(line))) {
            found.push((name.text.to_string(), body));
        }
    }
    found
}

/// Whether a line is the attribute that makes a function a test: `#[test]`, and
/// the runtimes that wrap it.
fn is_test_attribute(code: &str) -> bool {
    let Some(inside) = code
        .trim()
        .strip_prefix(ATTRIBUTE)
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    let path = inside.split('(').next().unwrap_or(inside).trim();
    path.rsplit("::").next().unwrap_or(path) == TEST_ATTRIBUTE
}

/// Where a macro is invoked, and the column its entries open at. The lines the
/// macro was declared on are not an invocation of it.
fn invocations(mask: &Mask, name: &str, body: &Block) -> Vec<(u32, usize)> {
    let mut found = Vec::new();
    for at in 1..=mask.line_count() {
        if body.first <= at && at <= body.last {
            continue;
        }
        let code = mask.code(at);
        for word in words(&code) {
            if word.text != name || word.after != Some('!') {
                continue;
            }
            let after = code[..word.end].chars().count();
            if let Some(column) = code
                .chars()
                .enumerate()
                .skip(after)
                .find(|(_, character)| opening(*character) == 1)
                .map(|(column, _)| column)
            {
                found.push((at, column));
            }
        }
    }
    found
}

/// The last line of the block the first brace from a place opens, where the
/// brace is written outside every bracket the header opens before it: a
/// destructured binding or a call in the header opens nothing. A statement that
/// ends before any brace opens no block, except in Go, whose three-clause loop
/// header is written with the same semicolons.
fn body_after(lang: Lang, mask: &Mask, from: At) -> Option<u32> {
    let mut depth = 0_i32;
    for (at, character) in code_from(mask, from) {
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '{' if depth <= 0 => return closing_of(mask, at).map(|close| close.0),
            ';' if depth <= 0 && lang != Lang::Go => return None,
            _ => {}
        }
    }
    None
}

/// Where the bracket opening at a place closes.
fn closing_of(mask: &Mask, open: At) -> Option<At> {
    let mut depth = 0_i32;
    for (at, character) in code_from(mask, open) {
        depth += opening(character) - closing(character);
        if depth <= 0 {
            return (closing(character) == 1).then_some(at);
        }
    }
    None
}

/// The first code character from a place onward that is not a space, and where
/// it sits. A literal or a comment is blank in the code view, so this passes
/// over them too.
fn next_code(mask: &Mask, from: At) -> Option<(At, char)> {
    code_from(mask, from).find(|(_, character)| !character.is_whitespace())
}

/// The code character at a place.
fn char_at(mask: &Mask, at: At) -> Option<char> {
    mask.code(at.0).chars().nth(at.1)
}

/// The identifier starting at a place, and the column it ends at.
fn identifier(mask: &Mask, at: At) -> (String, usize) {
    let word: String = mask
        .code(at.0)
        .chars()
        .skip(at.1)
        .take_while(|character| is_identifier(*character))
        .collect();
    let end = at.1 + word.chars().count();
    (word, end)
}

/// Every code character from a place onward, line after line, with where each
/// one sits.
fn code_from(mask: &Mask, from: At) -> impl Iterator<Item = (At, char)> + '_ {
    (from.0..=mask.line_count()).flat_map(move |line| {
        let start = if line == from.0 { from.1 } else { 0 };
        let characters: Vec<char> = mask.code(line).chars().collect();
        characters
            .into_iter()
            .enumerate()
            .skip(start)
            .map(move |(column, character)| ((line, column), character))
    })
}

/// The place one character on.
fn step(at: At) -> At {
    (at.0, at.1 + 1)
}

/// 1 for a character that opens a bracket, 0 for everything else.
fn opening(character: char) -> i32 {
    i32::from(matches!(character, '(' | '[' | '{'))
}

/// 1 for a character that closes a bracket, 0 for everything else.
fn closing(character: char) -> i32 {
    i32::from(matches!(character, ')' | ']' | '}'))
}
