//! Where a line is code, and where it is a comment or a literal.
//!
//! A rule that looks for a token has to know whether the token is the program
//! or something the program merely says: `#[ignore]` in a comment is a note, a
//! work marker in a string is a message, and either one in code is a finding.
//! weed parses nothing to decide that. The delimiters each language spells its
//! comments and its literals with are enough to tell the three apart, and the
//! scan is total — a file that ends inside a string simply ends there.
//!
//! The same scan gives a rule its words: an identifier with the character
//! before it and the one after it, which is what separates `skip` the member
//! from `skip` inside `skipped`.

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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mask {
    lines: Vec<Vec<(char, Syntax)>>,
}

impl Mask {
    /// The file scanned as the language it is written in. A language weed has
    /// no delimiters for reads as code throughout, which is the honest answer:
    /// weed does not know where its strings end.
    #[must_use]
    pub fn of(lang: Lang, content: &str) -> Mask {
        let dialect = dialect(lang);
        let mut state = State::Code;
        let mut lines = Vec::new();
        for line in content.lines() {
            let characters: Vec<char> = line.chars().collect();
            let mut kinds = vec![Syntax::Code; characters.len()];
            state = scan(&dialect, &characters, &mut kinds, state);
            lines.push(characters.into_iter().zip(kinds).collect());
        }
        Mask { lines }
    }

    /// The 1-based line with everything that is not code blanked.
    #[must_use]
    pub fn code(&self, line: u32) -> String {
        self.view(line, |syntax| syntax == Syntax::Code)
    }

    /// The 1-based line with its literals blanked. A comment is still there,
    /// which is where a work marker belongs and where it still counts.
    #[must_use]
    pub fn outside_literals(&self, line: u32) -> String {
        self.view(line, |syntax| syntax != Syntax::Literal)
    }

    /// The 1-based line with its comments blanked. What is left is the whole of
    /// the program: a literal counts here, because a function returning one is
    /// returning something.
    #[must_use]
    pub fn outside_comments(&self, line: u32) -> String {
        self.view(line, |syntax| syntax != Syntax::Comment)
    }

    /// What the 1-based line's literals hold, with the code and the comments
    /// blanked. This is what the program says, as opposed to what it does.
    #[must_use]
    pub fn literals(&self, line: u32) -> String {
        self.view(line, |syntax| syntax == Syntax::Literal)
    }

    fn view(&self, line: u32, keep: impl Fn(Syntax) -> bool) -> String {
        let Some(index) = (line as usize).checked_sub(1) else {
            return String::new();
        };
        self.lines
            .get(index)
            .map(|characters| {
                characters
                    .iter()
                    .map(|(character, syntax)| if keep(*syntax) { *character } else { ' ' })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// One identifier of a line, with the characters that decide what it is: a `.`
/// before it makes it a member, a `(` after it makes it a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word<'a> {
    pub text: &'a str,
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

/// Whether a character can sit inside an identifier in the languages weed reads.
#[must_use]
pub fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

fn word(line: &str, from: usize, to: usize) -> Word<'_> {
    Word {
        text: &line[from..to],
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
    /// closes again — anything else is a lifetime.
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
