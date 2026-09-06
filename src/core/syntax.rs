//! Where a line is code, and where it is a comment or a literal.
//!
//! A rule that looks for a token has to know whether the token is the program
//! or something the program merely says: `#[ignore]` in a comment is a note, a
//! work marker in a string is a message, and either one in code is a finding.
//! weeder parses nothing to decide that. The delimiters each language spells its
//! comments and its literals with are enough to tell the three apart, and the
//! scan is total, a file that ends inside a string simply ends there.
//!
//! The same scan gives a rule its words: an identifier with the character
//! before it and the one after it, which is what separates `skip` the member
//! from `skip` inside `skipped`.

use std::borrow::Cow;

use crate::core::classify::Lang;

/// What a character belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Code,
    Comment,
    /// A string, a character or a template literal, delimiters included.
    Literal,
}

/// A file read once, so a rule can ask any of its lines what it is made of.
///
/// What the scan keeps is where each thing starts rather than what every
/// character is: a line, and the runs of code, comment and literal it is made
/// of. A file is nearly all code and its lines carry a run or two, so this is
/// the file itself and a little over, where a kind per character would be
/// several times the file. The difference is the difference between reading a
/// twenty mebibyte file and holding it several times over.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mask {
    /// The file as it was scanned. The scan is only ever read against the text
    /// it was made from, so the two live together.
    content: String,
    lines: Vec<Line>,
    /// Every line's runs, one after another. A line names where its own begin.
    runs: Vec<Run>,
}

/// One line of the file: where it sits in the content, and where its runs begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Line {
    start: usize,
    end: usize,
    first_run: usize,
}

/// Where one run of a single kind begins, counted in characters from the start
/// of the line. The run reaches to the next one, or to the end of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    from: usize,
    syntax: Syntax,
}

impl Mask {
    /// The file scanned as the language it is written in. A language weeder has
    /// no delimiters for reads as code throughout, which is the honest answer:
    /// weeder does not know where its strings end.
    #[must_use]
    pub fn of(lang: Lang, content: &str) -> Mask {
        let dialect = dialect(lang);
        let mut state = State::Code;
        let mut lines = Vec::new();
        let mut runs = Vec::new();
        // One buffer for the whole file rather than one per line: the scan
        // needs a line as characters, and a file is a great many lines.
        let mut characters: Vec<char> = Vec::new();
        let mut kinds: Vec<Syntax> = Vec::new();
        let mut start = 0;
        for raw in content.split_inclusive('\n') {
            let line = written(raw);
            characters.clear();
            characters.extend(line.chars());
            kinds.clear();
            kinds.resize(characters.len(), Syntax::Code);
            state = scan(&dialect, &characters, &mut kinds, state);
            lines.push(Line {
                start,
                end: start + line.len(),
                first_run: runs.len(),
            });
            let mut open: Option<Syntax> = None;
            for (at, kind) in kinds.iter().enumerate() {
                if open != Some(*kind) {
                    runs.push(Run {
                        from: at,
                        syntax: *kind,
                    });
                    open = Some(*kind);
                }
            }
            start += raw.len();
        }
        Mask {
            content: content.to_string(),
            lines,
            runs,
        }
    }

    /// The 1-based line with everything that is not code blanked.
    #[must_use]
    pub fn code(&self, line: u32) -> Cow<'_, str> {
        self.view(line, |syntax| syntax == Syntax::Code)
    }

    /// The 1-based line with its literals blanked. A comment is still there,
    /// which is where a work marker belongs and where it still counts.
    #[must_use]
    pub fn outside_literals(&self, line: u32) -> Cow<'_, str> {
        self.view(line, |syntax| syntax != Syntax::Literal)
    }

    /// The 1-based line with its comments blanked. What is left is the whole of
    /// the program: a literal counts here, because a function returning one is
    /// returning something.
    #[must_use]
    pub fn outside_comments(&self, line: u32) -> Cow<'_, str> {
        self.view(line, |syntax| syntax != Syntax::Comment)
    }

    /// What the 1-based line's comments hold, with the code and the literals
    /// blanked. This is what the author said to the next reader, which is how a
    /// rule tells a body left empty by accident from one left empty on purpose.
    #[must_use]
    pub fn comments(&self, line: u32) -> Cow<'_, str> {
        self.view(line, |syntax| syntax == Syntax::Comment)
    }

    /// What the 1-based line's literals hold, with the code and the comments
    /// blanked. This is what the program says, as opposed to what it does.
    #[must_use]
    pub fn literals(&self, line: u32) -> Cow<'_, str> {
        self.view(line, |syntax| syntax == Syntax::Literal)
    }

    /// The literals of the 1-based line, one at a time, delimiters included and
    /// each with the character it starts at. A literal holds spaces of its own,
    /// so a caller that needs them apart cannot get them by splitting
    /// [`Mask::literals`], the run is the boundary, and the count of characters
    /// is what a caller compares against, because a blanked view is the same
    /// characters and not the same bytes.
    #[must_use]
    pub fn literal_runs(&self, line: u32) -> Vec<Literal> {
        let mut runs: Vec<Literal> = Vec::new();
        let mut open: Option<Literal> = None;
        for (at, (character, syntax)) in self.characters(line).enumerate() {
            match (syntax == Syntax::Literal, &mut open) {
                (true, Some(literal)) => literal.text.push(character),
                (true, None) => {
                    open = Some(Literal {
                        start: at,
                        text: character.to_string(),
                    })
                }
                (false, _) => runs.extend(open.take()),
            }
        }
        runs.extend(open);
        runs
    }

    /// How many lines the file has.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        u32::try_from(self.lines.len()).unwrap_or(u32::MAX)
    }

    /// A view of one line: the characters this kind of view keeps, and a space
    /// for every one it does not. Nearly every line of nearly every file is one
    /// run of code, and a view of such a line is the line itself, which is why
    /// this hands back what it was already holding rather than a copy of it.
    fn view(&self, line: u32, keep: impl Fn(Syntax) -> bool) -> Cow<'_, str> {
        let (text, runs) = self.at(line);
        if let [only] = runs {
            return match keep(only.syntax) {
                true => Cow::Borrowed(text),
                false => Cow::Owned(text.chars().map(|_| ' ').collect()),
            };
        }
        Cow::Owned(
            self.characters(line)
                .map(|(character, syntax)| if keep(syntax) { character } else { ' ' })
                .collect(),
        )
    }

    /// The 1-based line, character by character, each with what it belongs to.
    /// A line the file does not have is no characters at all.
    fn characters(&self, line: u32) -> impl Iterator<Item = (char, Syntax)> + '_ {
        let (text, runs) = self.at(line);
        let mut next = 0;
        let mut open = Syntax::Code;
        text.chars().enumerate().map(move |(at, character)| {
            while runs.get(next).is_some_and(|run| run.from <= at) {
                open = runs[next].syntax;
                next += 1;
            }
            (character, open)
        })
    }

    /// One 1-based line as it is written, and the runs it is made of.
    fn at(&self, line: u32) -> (&str, &[Run]) {
        let Some(index) = (line as usize).checked_sub(1) else {
            return ("", &[]);
        };
        let Some(line) = self.lines.get(index) else {
            return ("", &[]);
        };
        let last = self
            .lines
            .get(index + 1)
            .map_or(self.runs.len(), |next| next.first_run);
        (
            &self.content[line.start..line.end],
            &self.runs[line.first_run..last],
        )
    }
}

/// One line as `str::lines` reads it: what `split_inclusive` handed over,
/// without the newline that ends it, and without the carriage return a windows
/// editor puts in front of that.
fn written(raw: &str) -> &str {
    match raw.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => raw,
    }
}

/// One literal a line holds, and where in the line it begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    /// How many characters of the line come before it.
    pub start: usize,
    /// The literal as it is written, delimiters included.
    pub text: String,
}

/// One identifier of a line, with the characters that decide what it is: a `.`
/// before it makes it a member, a `(` after it makes it a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word<'a> {
    pub text: &'a str,
    /// Where the word begins in the line, in bytes.
    pub start: usize,
    /// Where it ends.
    pub end: usize,
    /// The nearest character before the word that is not a space.
    pub before: Option<char>,
    /// The nearest character after the word that is not a space.
    pub after: Option<char>,
}

impl Word<'_> {
    /// Whether the word is reached through a `.`, as `it.skip` reaches `skip`.
    #[must_use]
    pub fn is_member(&self) -> bool {
        self.before == Some('.')
    }

    /// Whether the word is called, as `xit(` is called.
    #[must_use]
    pub fn is_called(&self) -> bool {
        self.after == Some('(')
    }
}

/// The identifiers a line carries, in the order they are written.
#[must_use]
pub fn words(line: &str) -> Vec<Word<'_>> {
    let mut found = Vec::new();
    let mut start: Option<usize> = None;
    for (index, character) in line.char_indices() {
        if is_identifier(character) {
            start.get_or_insert(index);
            continue;
        }
        if let Some(from) = start.take() {
            found.push(word(line, from, index));
        }
    }
    if let Some(from) = start {
        found.push(word(line, from, line.len()));
    }
    found
}

/// Whether a character can sit inside an identifier in the languages weeder reads.
#[must_use]
pub fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

fn word(line: &str, from: usize, to: usize) -> Word<'_> {
    Word {
        text: &line[from..to],
        start: from,
        end: to,
        before: line[..from].chars().rev().find(|c| !c.is_whitespace()),
        after: line[to..].chars().find(|c| !c.is_whitespace()),
    }
}

/// How a language spells its comments and its literals.
#[derive(Debug, Clone, Copy)]
struct Dialect {
    /// `#` runs to the end of the line.
    hash_comment: bool,
    /// `//` runs to the end of the line.
    slash_comment: bool,
    /// `/*` runs to `*/`.
    block_comment: bool,
    /// A `/*` inside a block comment opens another one.
    nested_block: bool,
    /// `"""` and `'''` run across lines.
    triple_quotes: bool,
    /// A letter before the quote may turn escapes off, as Python's `r` does.
    raw_prefix: bool,
    /// A `"` string may run across lines.
    multiline_double: bool,
    /// What a backtick opens, where the language has one.
    backtick: Option<Backtick>,
    /// A `'` opens a character literal rather than a string, and only where it
    /// closes again, anything else is a lifetime.
    character_literal: bool,
    /// `r"…"`, `r#"…"#`, `br##"…"##`: no escapes, and the hashes decide the end.
    hashed_raw: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backtick {
    /// A template literal: escapes count, and it runs across lines.
    Template,
    /// A raw string: a backslash is a backslash, and it runs across lines.
    Raw,
}

fn dialect(lang: Lang) -> Dialect {
    let none = Dialect {
        hash_comment: false,
        slash_comment: false,
        block_comment: false,
        nested_block: false,
        triple_quotes: false,
        raw_prefix: false,
        multiline_double: false,
        backtick: None,
        character_literal: false,
        hashed_raw: false,
    };
    match lang {
        Lang::TypeScript | Lang::JavaScript => Dialect {
            slash_comment: true,
            block_comment: true,
            backtick: Some(Backtick::Template),
            ..none
        },
        Lang::Python => Dialect {
            hash_comment: true,
            triple_quotes: true,
            raw_prefix: true,
            ..none
        },
        Lang::Rust => Dialect {
            slash_comment: true,
            block_comment: true,
            nested_block: true,
            multiline_double: true,
            character_literal: true,
            hashed_raw: true,
            ..none
        },
        Lang::Go => Dialect {
            slash_comment: true,
            block_comment: true,
            backtick: Some(Backtick::Raw),
            character_literal: true,
            ..none
        },
        Lang::Other => none,
    }
}

/// Where the scan is between two characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Code,
    /// Inside a block comment, and how many are open.
    Block(u32),
    Literal {
        closer: Closer,
        /// Whether a backslash makes the next character part of the literal.
        escapes: bool,
        /// Whether the literal may reach the next line.
        spans_lines: bool,
    },
}

/// What ends a literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closer {
    /// This character, once.
    Once(char),
    /// This character, three times over.
    Thrice(char),
    /// A `"` followed by this many `#`.
    Hashes(u32),
}

/// One line classified, and the state the next line starts in. A literal that
/// cannot reach the next line ends where the line does, so an apostrophe in a
/// comment never swallows the rest of the file.
fn scan(dialect: &Dialect, characters: &[char], kinds: &mut [Syntax], start: State) -> State {
    let mut state = start;
    let mut index = 0;
    while index < characters.len() {
        state = match state {
            State::Block(depth) => {
                let (next, step) = in_block(dialect, characters, kinds, index, depth);
                index += step;
                next
            }
            State::Literal {
                closer,
                escapes,
                spans_lines,
            } => {
                let (next, step) =
                    in_literal(characters, kinds, index, closer, escapes, spans_lines);
                index += step;
                next
            }
            State::Code => {
                let (next, step) = in_code(dialect, characters, kinds, index);
                index += step;
                next
            }
        };
    }
    match state {
        State::Literal {
            spans_lines: false, ..
        } => State::Code,
        other => other,
    }
}

fn in_block(
    dialect: &Dialect,
    characters: &[char],
    kinds: &mut [Syntax],
    index: usize,
    depth: u32,
) -> (State, usize) {
    kinds[index] = Syntax::Comment;
    if dialect.nested_block && pair(characters, index, '/', '*') {
        kinds[index + 1] = Syntax::Comment;
        return (State::Block(depth + 1), 2);
    }
    if pair(characters, index, '*', '/') {
        kinds[index + 1] = Syntax::Comment;
        let next = if depth <= 1 {
            State::Code
        } else {
            State::Block(depth - 1)
        };
        return (next, 2);
    }
    (State::Block(depth), 1)
}

fn in_literal(
    characters: &[char],
    kinds: &mut [Syntax],
    index: usize,
    closer: Closer,
    escapes: bool,
    spans_lines: bool,
) -> (State, usize) {
    kinds[index] = Syntax::Literal;
    let open = State::Literal {
        closer,
        escapes,
        spans_lines,
    };
    if escapes && characters[index] == '\\' {
        if let Some(kind) = kinds.get_mut(index + 1) {
            *kind = Syntax::Literal;
        }
        return (open, 2);
    }
    match closer {
        Closer::Once(quote) if characters[index] == quote => (State::Code, 1),
        Closer::Thrice(quote)
            if characters[index] == quote
                && characters.get(index + 1) == Some(&quote)
                && characters.get(index + 2) == Some(&quote) =>
        {
            kinds[index + 1] = Syntax::Literal;
            kinds[index + 2] = Syntax::Literal;
            (State::Code, 3)
        }
        Closer::Hashes(hashes) if characters[index] == '"' => {
            let closed =
                (1..=hashes as usize).all(|step| characters.get(index + step) == Some(&'#'));
            if !closed {
                return (open, 1);
            }
            for step in 1..=hashes as usize {
                kinds[index + step] = Syntax::Literal;
            }
            (State::Code, 1 + hashes as usize)
        }
        _ => (open, 1),
    }
}

fn in_code(
    dialect: &Dialect,
    characters: &[char],
    kinds: &mut [Syntax],
    index: usize,
) -> (State, usize) {
    let character = characters[index];

    if (dialect.hash_comment && character == '#')
        || (dialect.slash_comment && pair(characters, index, '/', '/'))
    {
        for kind in kinds.iter_mut().skip(index) {
            *kind = Syntax::Comment;
        }
        return (State::Code, characters.len() - index);
    }
    if dialect.block_comment && pair(characters, index, '/', '*') {
        kinds[index] = Syntax::Comment;
        kinds[index + 1] = Syntax::Comment;
        return (State::Block(1), 2);
    }
    if dialect.triple_quotes
        && matches!(character, '"' | '\'')
        && characters.get(index + 1) == Some(&character)
        && characters.get(index + 2) == Some(&character)
    {
        for step in 0..3 {
            kinds[index + step] = Syntax::Literal;
        }
        return (
            State::Literal {
                closer: Closer::Thrice(character),
                escapes: !(dialect.raw_prefix && raw_prefixed(characters, index)),
                spans_lines: true,
            },
            3,
        );
    }
    if dialect.hashed_raw {
        if let Some((hashes, body)) = hashed_raw_open(characters, index) {
            for kind in kinds.iter_mut().take(body).skip(index) {
                *kind = Syntax::Literal;
            }
            return (
                State::Literal {
                    closer: Closer::Hashes(hashes),
                    escapes: false,
                    spans_lines: true,
                },
                body - index,
            );
        }
    }
    if character == '"' {
        kinds[index] = Syntax::Literal;
        return (
            State::Literal {
                closer: Closer::Once('"'),
                escapes: !(dialect.raw_prefix && raw_prefixed(characters, index)),
                spans_lines: dialect.multiline_double,
            },
            1,
        );
    }
    if character == '\'' {
        if dialect.character_literal && !is_character_literal(characters, index) {
            // A lifetime, not a literal: it never closes, and reading it as a
            // literal would swallow the rest of the file.
            return (State::Code, 1);
        }
        kinds[index] = Syntax::Literal;
        return (
            State::Literal {
                closer: Closer::Once('\''),
                escapes: !(dialect.raw_prefix && raw_prefixed(characters, index)),
                spans_lines: false,
            },
            1,
        );
    }
    if character == '`' {
        if let Some(backtick) = dialect.backtick {
            kinds[index] = Syntax::Literal;
            return (
                State::Literal {
                    closer: Closer::Once('`'),
                    escapes: backtick == Backtick::Template,
                    spans_lines: true,
                },
                1,
            );
        }
    }
    (State::Code, 1)
}

fn pair(characters: &[char], index: usize, first: char, second: char) -> bool {
    characters[index] == first && characters.get(index + 1) == Some(&second)
}

/// Whether the quote at `index` is opened by a prefix that turns escapes off:
/// Python's `r`, and the combinations it is written in.
fn raw_prefixed(characters: &[char], index: usize) -> bool {
    let mut start = index;
    while start > 0 && characters[start - 1].is_ascii_alphabetic() && index - start < 3 {
        start -= 1;
    }
    if start > 0 && is_identifier(characters[start - 1]) {
        return false;
    }
    characters[start..index]
        .iter()
        .any(|character| matches!(character, 'r' | 'R'))
}

/// Where a raw string opened at `index` starts its body, and how many hashes
/// close it. `None` where `index` opens no raw string at all.
fn hashed_raw_open(characters: &[char], index: usize) -> Option<(u32, usize)> {
    if index > 0 && is_identifier(characters[index - 1]) {
        return None;
    }
    let mut at = index;
    if characters.get(at) == Some(&'b') {
        at += 1;
    }
    if characters.get(at) != Some(&'r') {
        return None;
    }
    at += 1;
    let mut hashes = 0;
    while characters.get(at) == Some(&'#') {
        hashes += 1;
        at += 1;
    }
    (characters.get(at) == Some(&'"')).then_some((hashes, at + 1))
}

/// Whether the `'` at `index` opens a character literal. Rust spells a lifetime
/// with the same character and never closes it, so the test is whether a close
/// arrives where a character literal would put one.
fn is_character_literal(characters: &[char], index: usize) -> bool {
    match characters.get(index + 1) {
        Some('\\') => characters
            .iter()
            .skip(index + 2)
            .take(8)
            .any(|character| *character == '\''),
        Some(_) => characters.get(index + 2) == Some(&'\''),
        None => false,
    }
}
