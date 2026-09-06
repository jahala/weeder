//! The test cases a file generates rather than writes out.
//!
//! A suite that grows stops declaring its cases one at a time. Three `it` calls
//! become a table and a `for` around one call; three Go tests become one test
//! with a `t.Run` in a range; three Python functions become one function and a
//! list the decorator is handed; three Rust tests become a macro invoked once
//! per line. Nothing is covered less afterwards, and a reader that counts
//! declarations counts the difference as a deletion.
//!
//! So weed counts the table instead. A declaration written inside a loop stands
//! for one case per entry in what the loop runs over; a test that runs subtests
//! over a table stands for one per entry too; a declaration handed a table by
//! the decorator above it stands for one per entry; and a macro whose body
//! carries a test attribute gives one case for each entry it is invoked with.
//! Where the entries cannot be counted, a generated declaration stands for the
//! one case that is written, which is what the file says on its face.
//!
//! Some of those declarations the reader never reports at all: a title that is
//! a variable rather than a literal, a function a decorator stands in front of.
//! Those are read off the lines that declare them, so a case counts whether or
//! not the substrate could name it.
//!
//! Everything is read through the syntax mask, so a comma inside a string never
//! ends an entry and a brace inside a comment never opens a table.

use crate::core::change::Side;
use crate::core::classify::Lang;
use crate::core::read::TestUnit;
use crate::core::rules::check::idiom::{self, Block};
use crate::core::rules::check::vocab::Suite;
use crate::core::syntax::{is_identifier, words, Mask};

/// The keyword every language weed reads opens a loop over a collection with.
const LOOP_KEYWORD: &str = "for";

/// The words a language writes between a loop's binding and what it runs over.
const RUNS_OVER: &[&str] = &["of", "in", "range"];

/// The names that declare a case where the language spells one as a call. The
/// words that group cases rather than declare one are not here: a suite that
/// holds nothing is not a case.
const CASE_CALLS: &[&str] = &["it", "test", "bench"];

/// What a Python test function's name begins with, which is how its runners
/// collect one.
const PYTHON_CASE: &str = "test";

/// The word Python writes in front of a function it is declaring.
const PYTHON_DECLARATION: &str = "def";

/// The character a decorator opens with.
const DECORATOR: char = '@';

/// What Rust writes in front of a macro it is declaring.
const MACRO_KEYWORD: &str = "macro_rules";

/// The attribute that makes a function a test, whichever runtime wraps it.
const TEST_ATTRIBUTE: &str = "test";

/// One loop, and the lines it runs over.
struct Loop {
    at: u32,
    body: Block,
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

    let mut total = 0;
    for case in &declared {
        total += subtests(mask, &suite, &loops, case)
            .unwrap_or_else(|| stands_for(mask, &loops, case.start_line));
    }
    for line in unseen(lang, mask, &declared) {
        total += stands_for(mask, &loops, line);
    }
    total + macro_generated(lang, mask)
}

/// How many cases one declaration stands for: the entries of the table driving
/// it, or the one case that is written.
fn stands_for(mask: &Mask, loops: &[Loop], line: u32) -> usize {
    around(mask, loops, line)
        .or_else(|| decorated(mask, line))
        .unwrap_or(1)
}

/// The entries of the table the innermost loop around a declaration runs over.
fn around(mask: &Mask, loops: &[Loop], line: u32) -> Option<usize> {
    loops
        .iter()
        .filter(|opened| opened.at < line && line <= opened.body.last)
        .max_by_key(|opened| opened.at)
        .and_then(|opened| table(mask, opened.at))
}

/// The entries of the table a test runs subtests over: a loop written inside
/// the case, whose body opens a test inside the test.
fn subtests(mask: &Mask, suite: &Suite, loops: &[Loop], case: &TestUnit) -> Option<usize> {
    loops
        .iter()
        .filter(|opened| opened.at > case.start_line && opened.body.last <= case.end_line)
        .find(|opened| (opened.at..=opened.body.last).any(|line| suite.opens_subtest(mask, line)))
        .and_then(|opened| table(mask, opened.at))
}

/// The entries of the table a decorator directly above a declaration hands it.
///
/// The search walks up from the declaration, closing brackets as it goes, so a
/// decorator written across several lines is followed to the line that opened
/// it and an ordinary statement above ends the search at once.
fn decorated(mask: &Mask, line: u32) -> Option<usize> {
    let mut depth = 0_i32;
    let mut found = None;
    for above in (1..line).rev() {
        let code = mask.code(above);
        for character in code.chars() {
            depth += closing(character) - opening(character);
        }
        if depth > 0 {
            continue;
        }
        if code.trim_start().starts_with(DECORATOR) {
            found = Some(above);
        }
        break;
    }
    let opened = found?;
    (opened..line).find_map(|at| {
        let column = mask
            .code(at)
            .chars()
            .position(|character| character == '[')?;
        entries_at(mask, at, column)
    })
}

/// Every loop the file opens, with the lines each one runs over.
fn loops(lang: Lang, mask: &Mask) -> Vec<Loop> {
    let mut found = Vec::new();
    for at in 1..=mask.line_count() {
        let code = mask.code(at);
        let Some(after) = keyword_end(&code, LOOP_KEYWORD) else {
            continue;
        };
        let opener = match lang {
            // An indented language opens its body at the colon closing the header.
            Lang::Python => after,
            _ => match body_brace(&code, after) {
                Some(column) => column,
                None => continue,
            },
        };
        if let Some(body) = idiom::block(lang, mask, at, opener) {
            found.push(Loop { at, body });
        }
    }
    found
}

/// Where the brace opening a block sits, counting from `from`. A brace inside
/// the header's own brackets, a destructured binding, opens nothing.
fn body_brace(code: &str, from: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (column, character) in code.chars().enumerate().skip(from) {
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '{' if depth <= 0 => return Some(column),
            _ => {}
        }
    }
    None
}

/// The entries of what the loop opened on a line runs over.
fn table(mask: &Mask, at: u32) -> Option<usize> {
    let code = mask.code(at);
    let after = words(&code)
        .into_iter()
        .rfind(|word| RUNS_OVER.contains(&word.text) && !word.is_member())
        .map(|word| code[..word.end].chars().count())?;
    expression_entries(mask, at, after)
}

/// The entries of the expression written from a column: the table itself where
/// one is written there, and what the name it gives holds where it is a name.
fn expression_entries(mask: &Mask, line: u32, from: usize) -> Option<usize> {
    let characters: Vec<char> = mask.code(line).chars().collect();
    let mut column = from;
    while characters.get(column).is_some_and(|c| c.is_whitespace()) {
        column += 1;
    }
    match characters.get(column) {
        Some('(' | '[' | '{') => entries_at(mask, line, column),
        Some(character) if is_identifier(*character) => {
            let mut end = column;
            while characters
                .get(end)
                .is_some_and(|c| is_identifier(*c) || *c == '.')
            {
                end += 1;
            }
            let path: String = characters[column..end].iter().collect();
            let name = path.rsplit('.').next().unwrap_or(&path).to_string();
            named_entries(mask, &name, line)
        }
        _ => None,
    }
}

/// The entries of the table a name was given, where the file gives it one above
/// the line that reads it.
fn named_entries(mask: &Mask, name: &str, before: u32) -> Option<usize> {
    (1..before).rev().find_map(|line| {
        let code = mask.code(line);
        let value = assigned_at(&code, name)?;
        assigned_entries(mask, line, value)
    })
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
        // A comparison joins nothing, and a type written between the name and
        // the operator brings brackets of its own.
        let compares = characters.get(column + 1) == Some(&'=')
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

/// The entries of the value an assignment gives, read as the last bracketed
/// group the statement opens. A type written in front of a composite literal
/// opens and closes before the value does, so the value is the last group.
fn assigned_entries(mask: &Mask, line: u32, from: usize) -> Option<usize> {
    let mut depth = 0_i32;
    let mut last: Option<(u32, usize)> = None;
    for at in line..=mask.line_count() {
        let code = mask.code(at);
        let start = if at == line { from } else { 0 };
        for (column, character) in code.chars().enumerate().skip(start) {
            if depth == 0 && opening(character) == 1 {
                last = Some((at, column));
            }
            depth += opening(character) - closing(character);
            // A statement that ends closes the value with it.
            if character == ';' && depth <= 0 {
                return last.and_then(|(line, column)| entries_at(mask, line, column));
            }
        }
        if depth <= 0 && last.is_some() {
            break;
        }
    }
    last.and_then(|(line, column)| entries_at(mask, line, column))
}

/// How many entries the bracketed group opening at a column holds: the things
/// separated by the commas written at the depth the bracket opened. A comma
/// closing the last entry ends it rather than opening another, and a group with
/// nothing in it holds nothing.
fn entries_at(mask: &Mask, line: u32, column: usize) -> Option<usize> {
    if opening(mask.code(line).chars().nth(column)?) != 1 {
        return None;
    }
    let mut depth = 0_i32;
    let mut commas = 0;
    let mut written = false;
    let mut trailing = false;
    for at in line..=mask.line_count() {
        let code = mask.code(at);
        let start = if at == line { column } else { 0 };
        for character in code.chars().skip(start) {
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
fn declares_a_case(lang: Lang, code: &str) -> bool {
    let spoken = words(code);
    match lang {
        Lang::TypeScript | Lang::JavaScript => spoken
            .iter()
            .any(|word| CASE_CALLS.contains(&word.text) && word.is_called() && !word.is_member()),
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
        .strip_prefix("#[")
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

/// Where a keyword ends, in characters from the start of the line.
fn keyword_end(code: &str, keyword: &str) -> Option<usize> {
    words(code)
        .into_iter()
        .find(|word| word.text == keyword && !word.is_member())
        .map(|word| code[..word.end].chars().count())
}

/// 1 for a character that opens a bracket, 0 for everything else.
fn opening(character: char) -> i32 {
    i32::from(matches!(character, '(' | '[' | '{'))
}

/// 1 for a character that closes a bracket, 0 for everything else.
fn closing(character: char) -> i32 {
    i32::from(matches!(character, ')' | ']' | '}'))
}
