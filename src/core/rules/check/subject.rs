//! What an assertion is about, and whether it pins the whole of it.
//!
//! T2 counts claims, and a count alone cannot tell three loose claims on one
//! value replaced by one claim that pins the whole value from two claims
//! dropped. `expect(text).toContain(a)` three times checks three things about
//! `text`; `expect(text).toBe(whole)` checks all of them and the rest besides.
//! So each assertion is read for its subject, the expression it is about, and
//! for whether it compares that subject whole: an exact equality in the
//! language's own vocabulary, `toBe`, `toEqual`, `toStrictEqual`, `==`,
//! `assertEqual`, `assert_eq!`, `Equal`, a Go test failing when `got != want`.
//!
//! A subject is the expression as written, together with what the case last
//! bound its leading name to. Two checks on `got` are checks on the same value
//! only when `got` holds the same thing for both, and a test that rebinds it
//! between them is checking two values under one name. A value compared whole
//! covers what is read off it too, `text.toLowerCase()` and `text.length`, and
//! a value read off another covers nothing of it: the length of a text says
//! nothing about which words it holds.
//!
//! Everything is read off one line through the syntax mask, so a bracket or an
//! operator inside a string never splits an argument. An assertion written
//! across several lines, or one this reading cannot take apart, has no subject,
//! and nothing covers it.

use crate::core::classify::Lang;
use crate::core::rules::check::generated::assigned_at;
use crate::core::rules::check::idiom;
use crate::core::rules::check::vocab::{names, parts, slack, Sense};
use crate::core::syntax::{is_identifier, Mask};

/// One assertion, as T2 follows it to the value it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The values the assertion reads: the subject of a loose claim, or both
    /// sides of an exact one. A side that is a literal is not a subject.
    pub subjects: Vec<Subject>,
    /// Whether the assertion compares its subjects whole.
    pub exact: bool,
}

/// One value an assertion reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// The expression, as written, without its spacing.
    pub written: String,
    /// What the case last bound the expression's leading name to, where it
    /// binds it at all.
    pub bound: Option<String>,
}

impl Subject {
    /// Whether comparing this value whole says everything a claim on the other
    /// one says: it is the same value, bound the same way, or the other is read
    /// off it through a member, an index or a call on it.
    #[must_use]
    pub fn covers(&self, other: &Subject) -> bool {
        self.bound == other.bound
            && other
                .written
                .strip_prefix(&self.written)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '[']))
    }
}

/// The words a matcher's name is built from in front of what it checks: the
/// root that makes it an assertion. `toBe` is `to` and `be`, and `be` is the
/// check.
const ROOTS: &[&str] = &["to", "assert", "debug", "expect", "should", "require"];

/// The last word of a matcher that compares two values whole.
const EQUALS: &[&str] = &["equal", "equals", "eq", "eql"];

/// The words that may stand in front of that last word and leave it whole:
/// how strictly, and over what kind of value.
const QUALIFIERS: &[&str] = &[
    "strict", "deep", "list", "dict", "tuple", "set", "sequence", "multi", "line",
];

/// The word a matcher is alone when it compares by identity, `toBe`.
const IDENTITY: &str = "be";

/// The word a matcher carries when it checks that a value is in another, and
/// reads its subject second: `assertIn(member, container)`.
const MEMBERSHIP: &str = "in";

/// The segment of a matcher chain that turns its check around.
const NEGATION: &str = "not";

/// The names a language gives a value it writes as a word. A side that is one
/// of them is a literal, like a number or a string.
const VALUE_WORDS: &[&str] = &[
    "nil",
    "null",
    "undefined",
    "None",
    "true",
    "false",
    "True",
    "False",
];

/// A stretch of a line, as columns counted in characters.
type Span = (usize, usize);

/// What one assertion reads, before its subjects are bound: the spans of the
/// line it is written on, and whether it compares them whole.
struct Shape {
    line: u32,
    spans: Vec<Span>,
    exact: bool,
}

/// One line as the two views read it: the code, with literals and comments
/// blanked, and the program, with only the comments blanked. They are the same
/// characters, so a column means the same thing in both.
struct View {
    code: Vec<char>,
    text: Vec<char>,
}

impl View {
    fn of(mask: &Mask, line: u32) -> View {
        View {
            code: mask.code(line).chars().collect(),
            text: mask.outside_comments(line).chars().collect(),
        }
    }

    fn code(&self, span: Span) -> String {
        self.code[span.0..span.1].iter().collect()
    }

    fn written(&self, span: Span) -> String {
        let text: String = self.text[span.0..span.1].iter().collect();
        text.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}

/// What the assertion on a 1-based line is about, where it can be read: the
/// subjects, each bound to what the case gave its leading name at or above the
/// line, searching no higher than the line the case opens on.
#[must_use]
pub fn read(lang: Lang, mask: &Mask, line: u32, floor: u32) -> Option<Reading> {
    let shape = match lang {
        Lang::TypeScript | Lang::JavaScript => {
            script(&View::of(mask, line)).map(|(spans, exact)| Shape { line, spans, exact })
        }
        Lang::Python => {
            python(&View::of(mask, line)).map(|(spans, exact)| Shape { line, spans, exact })
        }
        Lang::Rust => {
            rust(&View::of(mask, line)).map(|(spans, exact)| Shape { line, spans, exact })
        }
        Lang::Go => go(mask, line, floor),
        Lang::Other => None,
    }?;
    let view = View::of(mask, shape.line);
    let subjects: Vec<Subject> = shape
        .spans
        .into_iter()
        .map(|span| trimmed(&view.code, span))
        .filter(|span| span.0 < span.1 && !is_literal(&view.code(*span)))
        .map(|span| bound(mask, &view, span, shape.line, floor))
        .collect();
    (!subjects.is_empty()).then_some(Reading {
        subjects,
        exact: shape.exact,
    })
}

/// A subject as written, with what its leading name was last bound to, where
/// the case binds it.
fn bound(mask: &Mask, view: &View, span: Span, line: u32, floor: u32) -> Subject {
    let written = view.written(span);
    let root: String = view.code[span.0..span.1]
        .iter()
        .take_while(|character| is_identifier(**character))
        .collect();
    if root.is_empty() {
        return Subject {
            written,
            bound: None,
        };
    }
    let binding = (floor.min(line)..=line).rev().find_map(|at| {
        let code = mask.code(at);
        let column = assigned_at(&code, &root)?;
        let bound = View::of(mask, at);
        let end = top_level(&bound.code, (column, bound.code.len()), &[";"])
            .map_or(bound.code.len(), |(at, _)| at);
        Some(bound.written((column, end)))
    });
    Subject {
        written,
        bound: binding,
    }
}

/// Whether a matcher's name compares two values whole.
fn pins_whole(name: &str) -> bool {
    let words = parts(name);
    let rest: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .skip_while(|word| ROOTS.contains(word))
        .collect();
    match rest.split_last() {
        Some((last, [])) if *last == IDENTITY => true,
        Some((last, front)) => {
            EQUALS.contains(last) && front.iter().all(|word| QUALIFIERS.contains(word))
        }
        None => false,
    }
}

/// `expect(subject).matcher(expected)`, and the assertion libraries that hang
/// a matcher off one object: `assert.equal(actual, expected)`, `assert(ok)`.
fn script(view: &View) -> Option<(Vec<Span>, bool)> {
    let code = &view.code;
    let spelled: String = code.iter().collect();
    let named = names(&spelled);
    let first = named.iter().find(|name| name.is_called())?;
    let open = paren_after(code, columns(code, first.end))?;
    let close = close_of(code, open)?;
    if first.text == "expect" {
        let subject = (open + 1, close);
        let chain = chain_after(code, close);
        let (matcher, front) = chain.split_last()?;
        let exact = pins_whole(&matcher.0) && !front.iter().any(|(word, _)| word == NEGATION);
        if !exact {
            return Some((vec![subject], false));
        }
        let expected =
            paren_after(code, matcher.1).and_then(|open| arguments(code, open).into_iter().next());
        return Some((std::iter::once(subject).chain(expected).collect(), true));
    }
    if first.segments.first() != Some(&"assert") {
        return None;
    }
    let args = arguments(code, open);
    if first.segments.len() > 1 && pins_whole(first.last()) {
        return Some((args.into_iter().take(2).collect(), true));
    }
    predicate(code, *args.first()?, Lang::TypeScript)
}

/// `assert subject == expected`, `assert member in subject`, and the methods
/// that begin with the same word.
fn python(view: &View) -> Option<(Vec<Span>, bool)> {
    let code = &view.code;
    let start = code
        .iter()
        .position(|character| !character.is_whitespace())?;
    let keyword: String = code[start..]
        .iter()
        .take_while(|character| is_identifier(**character))
        .collect();
    if keyword == "assert" {
        let from = start + keyword.chars().count();
        let end = top_level(code, (from, code.len()), &[","]).map_or(code.len(), |(at, _)| at);
        return predicate(code, (from, end), Lang::Python);
    }
    let spelled: String = code.iter().collect();
    let named = names(&spelled);
    let name = named
        .iter()
        .find(|name| name.is_called() && name.last().starts_with("assert"))?;
    let open = paren_after(code, columns(code, name.end))?;
    let args = arguments(code, open);
    if pins_whole(name.last()) {
        return Some((args.into_iter().take(2).collect(), true));
    }
    if parts(name.last()).iter().any(|word| word == MEMBERSHIP) {
        return Some((vec![*args.get(1)?], false));
    }
    predicate(code, *args.first()?, Lang::Python)
}

/// `assert_eq!(left, right)`, `assert_ne!(left, right)`, `assert!(predicate)`.
fn rust(view: &View) -> Option<(Vec<Span>, bool)> {
    let code = &view.code;
    let spelled: String = code.iter().collect();
    let named = names(&spelled);
    let name = named
        .iter()
        .find(|name| name.is_macro() && name.last().contains("assert"))?;
    let bang = columns(code, name.end);
    let open = paren_after(code, bang + 1)?;
    let args = arguments(code, open);
    if pins_whole(name.last()) {
        return Some((args.into_iter().take(2).collect(), true));
    }
    let rest: Vec<String> = parts(name.last())
        .into_iter()
        .skip_while(|word| ROOTS.contains(&word.as_str()))
        .collect();
    if rest.is_empty() {
        return predicate(code, *args.first()?, Lang::Rust);
    }
    Some((vec![*args.first()?], false))
}

/// A Go test fails by telling its handle, under an `if` whose condition is the
/// failure: `got != want` asserts the two are equal. The assertion libraries
/// take the handle first and the values after it.
fn go(mask: &Mask, line: u32, floor: u32) -> Option<Shape> {
    let view = View::of(mask, line);
    let code = &view.code;
    let spelled: String = code.iter().collect();
    let named = names(&spelled);
    if let Some(name) = named
        .iter()
        .find(|name| name.is_called() && matches!(name.receiver(), Some("assert" | "require")))
    {
        let open = paren_after(code, columns(code, name.end))?;
        let args = arguments(code, open);
        let values: Vec<Span> = args.into_iter().skip(1).collect();
        return if pins_whole(name.last()) {
            Some(Shape {
                line,
                spans: values.into_iter().take(2).collect(),
                exact: true,
            })
        } else {
            Some(Shape {
                line,
                spans: vec![*values.first()?],
                exact: false,
            })
        };
    }
    let header = (floor.min(line)..line).rev().find(|at| {
        let code = mask.code(*at);
        let opens_if = code
            .trim_start()
            .trim_start_matches('}')
            .trim_start()
            .trim_start_matches("else")
            .trim_start()
            .starts_with("if ");
        opens_if && idiom::block(Lang::Go, mask, *at, 0).is_some_and(|block| line <= block.last)
    })?;
    let view = View::of(mask, header);
    let code = &view.code;
    let keyword = code.iter().collect::<String>().find("if ")?;
    let from = columns(code, keyword) + 3;
    let brace = (from..code.len()).rev().find(|at| code[*at] == '{')?;
    let condition = match top_level(code, (from, brace), &[";"]) {
        Some((at, _)) => (at + 1, brace),
        None => (from, brace),
    };
    let (spans, exact) = failure(code, condition)?;
    Some(Shape {
        line: header,
        spans,
        exact,
    })
}

/// What a Go condition that fails a test asserts: the opposite of itself.
fn failure(code: &[char], span: Span) -> Option<(Vec<Span>, bool)> {
    if top_level(code, span, &["&&", "||"]).is_some() {
        return None;
    }
    if let Some((at, operator)) = top_level(code, span, &["!=", "==", "<=", ">=", "<", ">"]) {
        let left = (span.0, at);
        let right = (at + operator.chars().count(), span.1);
        return Some(match operator {
            "!=" => (vec![left, right], true),
            _ => (vec![left], false),
        });
    }
    let span = trimmed(code, span);
    let negated = code.get(span.0) == Some(&'!');
    let inner = if negated { (span.0 + 1, span.1) } else { span };
    let (callee, args) = call(code, inner)?;
    let name = code[callee.0..callee.1].iter().collect::<String>();
    let last = name.rsplit('.').next().unwrap_or(&name).to_string();
    if negated && pins_whole(&last) {
        return Some((args.into_iter().take(2).collect(), true));
    }
    Some((vec![*args.first()?], false))
}

/// What a predicate asserts about its subject: a comparison, a call on the
/// subject, a function handed the subject, or the subject itself.
fn predicate(code: &[char], span: Span, lang: Lang) -> Option<(Vec<Span>, bool)> {
    // Python spells some operators as words with spaces around them, and a
    // literal before one is blanked to spaces, so the span is searched as it is
    // and each side trimmed afterwards.
    let joins: &[&str] = match lang {
        Lang::Python => &[" and ", " or "],
        _ => &["&&", "||"],
    };
    if top_level(code, span, joins).is_some() {
        return None;
    }
    let operators: &[&str] = match lang {
        Lang::Python => &[
            "==", "!=", "<=", ">=", "<", ">", " not in ", " in ", " is not ", " is ",
        ],
        _ => &["===", "!==", "==", "!=", "<=", ">=", "<", ">"],
    };
    if let Some((at, operator)) = top_level(code, span, operators) {
        let left = (span.0, at);
        let right = (at + operator.chars().count(), span.1);
        return Some(match operator {
            "===" | "==" if !near(code, right) && !near(code, left) => (vec![left, right], true),
            " in " | " not in " => (vec![right], false),
            _ => (vec![left], false),
        });
    }
    let mut inner = trimmed(code, span);
    loop {
        inner = trimmed(code, inner);
        if code.get(inner.0) == Some(&'!') {
            inner.0 += 1;
        } else if code[inner.0..inner.1].starts_with(&['n', 'o', 't', ' ']) {
            inner.0 += 4;
        } else {
            break;
        }
    }
    match call(code, inner) {
        Some((callee, args)) => match receiver(code, callee) {
            Some(subject) => Some((vec![subject], false)),
            None => Some((vec![*args.first()?], false)),
        },
        None => Some((vec![inner], false)),
    }
}

/// Whether a side of a comparison is a nearness, `pytest.approx(…)`, which
/// compares within a slack rather than whole.
fn near(code: &[char], span: Span) -> bool {
    names(&code[span.0..span.1].iter().collect::<String>())
        .iter()
        .any(|name| {
            name.segments
                .iter()
                .any(|segment| slack(segment) == Some(Sense::Nearness))
        })
}

/// A call that is the whole of a span: the callee and the arguments.
fn call(code: &[char], span: Span) -> Option<(Span, Vec<Span>)> {
    if span.1 == 0 || code.get(span.1 - 1) != Some(&')') {
        return None;
    }
    let mut depth = 0_i32;
    let mut open = None;
    for at in (span.0..span.1).rev() {
        depth += closing(code[at]) - opening(code[at]);
        if depth == 0 {
            open = Some(at);
            break;
        }
    }
    let open = open?;
    let callee = trimmed(code, (span.0, open));
    if callee.0 >= callee.1 {
        return None;
    }
    Some((callee, arguments(code, open)))
}

/// The value a method is called on, where the callee is a member of one.
fn receiver(code: &[char], callee: Span) -> Option<Span> {
    let mut depth = 0_i32;
    for at in (callee.0..callee.1).rev() {
        depth += closing(code[at]) - opening(code[at]);
        if depth == 0 && code[at] == '.' {
            return Some((callee.0, at));
        }
    }
    None
}

/// The first place at the top of a span where one of the operators is written,
/// and which one, the longer spellings tried first at every place.
fn top_level<'a>(code: &[char], span: Span, operators: &[&'a str]) -> Option<(usize, &'a str)> {
    let mut depth = 0_i32;
    for at in span.0..span.1 {
        let character = code[at];
        if depth == 0 {
            for operator in operators {
                let length = operator.chars().count();
                if at + length > span.1 {
                    continue;
                }
                if !code[at..at + length].iter().copied().eq(operator.chars()) {
                    continue;
                }
                // An arrow, a shift, or the end of a longer operator joins nothing.
                let before = at.checked_sub(1).map(|index| code[index]);
                let after = code.get(at + length).copied();
                let glued = matches!(after, Some('=' | '>' | '<'))
                    || matches!(before, Some('=' | '!' | '<' | '>' | '-'));
                if !glued || operator.starts_with(' ') {
                    return Some((at, operator));
                }
            }
        }
        depth += opening(character) - closing(character);
    }
    None
}

/// The arguments of the call whose parenthesis opens at a column, each as its
/// span.
fn arguments(code: &[char], open: usize) -> Vec<Span> {
    let Some(close) = close_of(code, open) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut depth = 0_i32;
    let mut start = open + 1;
    for (at, &character) in code.iter().enumerate().take(close).skip(open + 1) {
        if character == ',' && depth == 0 {
            found.push((start, at));
            start = at + 1;
        }
        depth += opening(character) - closing(character);
    }
    found.push((start, close));
    found
        .into_iter()
        .map(|span| trimmed(code, span))
        .filter(|span| span.0 < span.1)
        .collect()
}

/// The members a matcher chain reaches through after a call closes, each with
/// the column its name ends at.
fn chain_after(code: &[char], close: usize) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut at = close + 1;
    while code.get(at) == Some(&'.') {
        let word: String = code[at + 1..]
            .iter()
            .take_while(|character| is_identifier(**character))
            .collect();
        if word.is_empty() {
            break;
        }
        at += 1 + word.chars().count();
        found.push((word, at));
        if code.get(at) == Some(&'(') {
            match close_of(code, at) {
                Some(end) if code.get(end + 1) == Some(&'.') => at = end + 1,
                _ => break,
            }
        }
    }
    found
}

/// The column of the parenthesis a call opens at or after a column, passing
/// over spaces.
fn paren_after(code: &[char], from: usize) -> Option<usize> {
    let at = (from..code.len()).find(|at| !code[*at].is_whitespace())?;
    (code[at] == '(').then_some(at)
}

/// Where the bracket opening at a column closes on the same line.
fn close_of(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (at, character) in code.iter().enumerate().skip(open) {
        depth += opening(*character) - closing(*character);
        if depth == 0 {
            return Some(at);
        }
    }
    None
}

/// A span with the spaces at either end taken off.
fn trimmed(code: &[char], span: Span) -> Span {
    let mut span = span;
    while span.0 < span.1 && code[span.0].is_whitespace() {
        span.0 += 1;
    }
    while span.1 > span.0 && code[span.1 - 1].is_whitespace() {
        span.1 -= 1;
    }
    span
}

/// Whether a side is a literal: no name in it, once its strings are blanked,
/// or a value the language writes as a word.
fn is_literal(code: &str) -> bool {
    let code = code.trim();
    VALUE_WORDS.contains(&code)
        || !code
            .chars()
            .any(|character| character.is_alphabetic() || character == '_')
}

/// The column in characters that a byte offset of a line falls on.
fn columns(code: &[char], byte: usize) -> usize {
    let mut bytes = 0;
    for (at, character) in code.iter().enumerate() {
        if bytes >= byte {
            return at;
        }
        bytes += character.len_utf8();
    }
    code.len()
}

fn opening(character: char) -> i32 {
    i32::from(matches!(character, '(' | '[' | '{'))
}

fn closing(character: char) -> i32 {
    i32::from(matches!(character, ')' | ']' | '}'))
}

#[cfg(test)]
mod tests {
    use super::pins_whole;

    #[test]
    fn the_matchers_that_compare_whole_are_the_equalities_and_identity() {
        for name in [
            "toBe",
            "toEqual",
            "toStrictEqual",
            "assertEqual",
            "assertListEqual",
            "assert_eq",
            "debug_assert_eq",
            "Equal",
            "DeepEqual",
            "deepStrictEqual",
            "strictEqual",
            "equal",
        ] {
            assert!(pins_whole(name), "{name} compares whole");
        }
        for name in [
            "toContain",
            "toBeGreaterThan",
            "toMatch",
            "assertAlmostEqual",
            "assertCountEqual",
            "assertNotEqual",
            "assert_ne",
            "NotEqual",
            "Contains",
            "assert",
        ] {
            assert!(!pins_whole(name), "{name} does not compare whole");
        }
    }
}
