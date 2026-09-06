//! A file read the way the injector reads it, and nothing weed reads it with.
//!
//! The whole measurement turns on this file being independent. If the injector
//! asked weed where the test cases are, a rule that cannot see a case would
//! never be given one to miss, and recall would measure agreement rather than
//! detection. So the shapes here are the ones the languages' own runners
//! document: a `.test.ts` file, a `def test_` function, a `#[test]` attribute,
//! a `TestXxx` function. They are found with a small scanner that knows where a
//! line's code ends and its strings and comments begin, so a marker written
//! inside a string is never mistaken for one written in the code.

/// The languages the corpus is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    Ts,
    Py,
    Rs,
    Go,
}

impl Language {
    pub const ALL: [Language; 4] = [Language::Ts, Language::Py, Language::Rs, Language::Go];

    /// The word the report writes this language down as.
    pub fn slug(self) -> &'static str {
        match self {
            Language::Ts => "ts",
            Language::Py => "py",
            Language::Rs => "rs",
            Language::Go => "go",
        }
    }

    /// The extensions a file of this language wears. TypeScript answers for the
    /// JavaScript family too: one runner, one set of conventions.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Language::Ts => &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
            Language::Py => &["py"],
            Language::Rs => &["rs"],
            Language::Go => &["go"],
        }
    }

    /// Whether a path is a file of this language, leaving out the places a
    /// repository keeps other people's code and its own generated output.
    pub fn owns(self, path: &str) -> bool {
        if path.split('/').any(|segment| {
            matches!(
                segment,
                "node_modules" | "vendor" | "target" | "dist" | "build" | ".venv" | "site-packages"
            )
        }) {
            return false;
        }
        let name = basename(path);
        if name.ends_with(".d.ts") || name.ends_with(".min.js") {
            return false;
        }
        extension(path).is_some_and(|found| self.extensions().contains(&found))
    }

    /// The comment a line of this language is written as.
    pub fn comment(self, text: &str) -> String {
        match self {
            Language::Py => format!("# {text}"),
            _ => format!("// {text}"),
        }
    }

    /// Whether the runner collects a file at this path as a suite, by the
    /// convention the runner documents.
    pub fn collects_file(self, path: &str) -> bool {
        let name = basename(path);
        let directories: Vec<&str> = path.split('/').collect();
        let directories = &directories[..directories.len().saturating_sub(1)];
        match self {
            Language::Ts => {
                name.contains(".test.")
                    || name.contains(".spec.")
                    || directories.contains(&"__tests__")
            }
            Language::Py => {
                name.starts_with("test_")
                    || name
                        .strip_suffix(".py")
                        .is_some_and(|s| s.ends_with("_test"))
            }
            Language::Go => name.ends_with("_test.go"),
            // cargo builds an integration target from a file at the top of the
            // tests directory; anything one level down is a module nobody
            // compiles unless a target names it.
            Language::Rs => directories == ["tests"] || directories == ["benches"],
        }
    }
}

/// The extension of a path, without its dot.
pub fn extension(path: &str) -> Option<&str> {
    let name = basename(path);
    let (_, found) = name.rsplit_once('.')?;
    (!found.is_empty()).then_some(found)
}

pub fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(head, _)| head)
}

/// A file, its lines, and each line with everything that is not code taken out.
///
/// `code` is the line with string literals and comments replaced by spaces,
/// character for character, so a column in one is the column in the other. Every
/// search the injector makes runs over `code`; every edit it makes runs over
/// `lines`.
pub struct Source {
    pub path: String,
    pub lang: Language,
    pub lines: Vec<String>,
    pub code: Vec<String>,
}

impl Source {
    pub fn read(path: &str, lang: Language, text: &str) -> Source {
        let lines: Vec<String> = text.lines().map(ToString::to_string).collect();
        let code = mask(lang, &lines);
        Source {
            path: path.to_string(),
            lang,
            lines,
            code,
        }
    }

    pub fn text(&self) -> String {
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }

    /// Whether this file holds tests, by the runner's convention or, where the
    /// language writes its tests beside the code, by the marker it writes them
    /// with.
    pub fn holds_tests(&self) -> bool {
        if self.lang.collects_file(&self.path) {
            return true;
        }
        match self.lang {
            Language::Rs => self.code.iter().any(|line| line.contains("#[test]")),
            Language::Py => {
                let name = basename(&self.path);
                name.starts_with("test_") || name.ends_with("_test.py")
            }
            _ => false,
        }
    }

    /// The test cases the file declares: the line each one opens on, the line it
    /// closes on, and the name it carries.
    pub fn cases(&self) -> Vec<Case> {
        let mut cases = Vec::new();
        for (at, code) in self.code.iter().enumerate() {
            let Some(open) = self.case_opens(at, code) else {
                continue;
            };
            let Some(end) = self.block_end(at) else {
                continue;
            };
            if cases
                .last()
                .is_some_and(|last: &Case| last.last >= at as u32)
            {
                // A case declared inside another one is the inner runner's
                // business; the outer block is the case that was found first.
                continue;
            }
            cases.push(Case {
                name: open,
                first: at as u32,
                last: end as u32,
            });
        }
        cases
    }

    /// The name of the case a line opens, where it opens one.
    fn case_opens(&self, at: usize, code: &str) -> Option<String> {
        let trimmed = code.trim_start();
        match self.lang {
            Language::Ts => {
                let called = trimmed.split(['(', '.']).next()?;
                if !matches!(called, "it" | "test") {
                    return None;
                }
                // `it(` and `it.skip(`, but never `iterate(`.
                let after = trimmed.get(called.len()..)?.trim_start();
                if !after.starts_with('(') && !after.starts_with('.') {
                    return None;
                }
                Some(title(self.lines.get(at)?).unwrap_or_else(|| called.to_string()))
            }
            Language::Py => {
                let rest = trimmed.strip_prefix("def ")?;
                let name = rest.split(['(', ' ']).next()?;
                (name == "test" || name.starts_with("test_")).then(|| name.to_string())
            }
            Language::Rs => {
                if trimmed.trim_end() != "#[test]" {
                    return None;
                }
                let declaration = self
                    .code
                    .iter()
                    .skip(at + 1)
                    .find(|line| line.contains("fn "))?;
                let name = declaration.split("fn ").nth(1)?.split(['(', '<']).next()?;
                Some(name.trim().to_string())
            }
            Language::Go => {
                let rest = trimmed.strip_prefix("func ")?;
                let name = rest.split('(').next()?;
                (name.starts_with("Test")
                    || name.starts_with("Benchmark")
                    || name.starts_with("Fuzz"))
                .then(|| name.to_string())
            }
        }
    }

    /// The last line of the block a line opens, counted in the code alone. A
    /// language written in braces is counted by them; one written in
    /// indentation is counted by that.
    pub fn block_end(&self, first: usize) -> Option<usize> {
        match self.lang {
            Language::Py => Some(self.indented_end(first)),
            _ => self.braced_end(first),
        }
    }

    /// A block runs from the brace that opens it to the one that closes it.
    /// Only braces are counted: a handler is written `} catch (err) {`, and the
    /// brace it opens with is the third thing on the line.
    fn braced_end(&self, first: usize) -> Option<usize> {
        const LOOKING: usize = 6;
        let mut depth = 0i32;
        let mut opened = false;
        for (at, code) in self.code.iter().enumerate().skip(first) {
            for character in code.chars() {
                match character {
                    '{' => {
                        depth += 1;
                        opened = true;
                    }
                    '}' if depth > 0 => depth -= 1,
                    _ => {}
                }
            }
            if opened && depth == 0 {
                return Some(at);
            }
            // A declaration that opens nothing within a few lines is not a
            // block at all, and the next brace in the file belongs to something
            // else.
            if !opened && at > first + LOOKING {
                return None;
            }
            if at > first + 4000 {
                return None;
            }
        }
        None
    }

    /// The last line of a suite of lines indented under the first one, with the
    /// blank lines that trail it left behind.
    fn indented_end(&self, first: usize) -> usize {
        let indent = indent_of(self.code.get(first).map_or("", String::as_str));
        let mut last = first;
        for at in first + 1..self.code.len() {
            let line = &self.code[at];
            if line.trim().is_empty() {
                continue;
            }
            if indent_of(line) <= indent {
                break;
            }
            last = at;
        }
        last
    }

    /// The line a block's body opens on: the line after the one that opened it.
    pub fn body_first(&self, first: usize) -> Option<usize> {
        let end = self.block_end(first)?;
        (end > first).then_some(first + 1)
    }

    /// The indentation the body of a block is written at.
    pub fn body_indent(&self, first: usize) -> String {
        let outer = self.lines.get(first).map_or("", String::as_str);
        let inner = self
            .lines
            .get(first + 1)
            .filter(|line| !line.trim().is_empty())
            .map_or("", |line| line.as_str());
        let deeper = leading_whitespace(inner);
        if deeper.len() > leading_whitespace(outer).len() {
            deeper.to_string()
        } else {
            format!("{}{}", leading_whitespace(outer), self.step())
        }
    }

    fn step(&self) -> &'static str {
        match self.lang {
            Language::Go => "\t",
            Language::Rs => "    ",
            _ => "  ",
        }
    }

    /// The functions the file declares that no runner collects: where a stub, a
    /// print or a swallowed failure is planted.
    pub fn functions(&self) -> Vec<Case> {
        let mut found = Vec::new();
        for (at, code) in self.code.iter().enumerate() {
            let trimmed = code.trim_start();
            let opens = match self.lang {
                Language::Py => trimmed.starts_with("def ") || trimmed.starts_with("async def "),
                Language::Rs => {
                    trimmed.starts_with("fn ")
                        || trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("pub(crate) fn ")
                        || trimmed.starts_with("async fn ")
                        || trimmed.starts_with("pub async fn ")
                }
                Language::Go => trimmed.starts_with("func "),
                Language::Ts => {
                    (trimmed.starts_with("function ")
                        || trimmed.starts_with("export function ")
                        || trimmed.starts_with("async function ")
                        || trimmed.starts_with("export async function "))
                        && trimmed.contains('(')
                }
            };
            if !opens {
                continue;
            }
            let name = self.declared_name(trimmed).unwrap_or_default();
            let Some(end) = self.block_end(at) else {
                continue;
            };
            if end <= at {
                continue;
            }
            found.push(Case {
                name,
                first: at as u32,
                last: end as u32,
            });
        }
        found
    }

    fn declared_name(&self, trimmed: &str) -> Option<String> {
        let keyword = match self.lang {
            Language::Py => "def ",
            Language::Rs => "fn ",
            Language::Go => "func ",
            Language::Ts => "function ",
        };
        let rest = trimmed.split(keyword).nth(1)?;
        let name = rest.split(['(', '<', ' ']).next()?;
        (!name.is_empty()).then(|| name.to_string())
    }

    /// The lines the file makes an assertion on, one line each and balanced, so
    /// taking one out leaves the rest of the case as it was.
    pub fn assertions(&self) -> Vec<u32> {
        let claims: &[&str] = match self.lang {
            Language::Ts => &["expect(", "assert.", "assert(", ".should."],
            Language::Py => &["assert ", "self.assert", "assert("],
            Language::Rs => &["assert!(", "assert_eq!(", "assert_ne!("],
            Language::Go => &["t.Error", "t.Fatal", "assert.", "require."],
        };
        self.code
            .iter()
            .enumerate()
            .filter(|(_, code)| {
                let trimmed = code.trim_start();
                claims.iter().any(|claim| trimmed.starts_with(claim))
            })
            .filter(|(at, _)| balanced(&self.code[*at]))
            .map(|(at, _)| at as u32)
            .collect()
    }

    /// The import statements the file opens with, as line numbers, so a line
    /// planted after the last of them lands where the file already declares
    /// what it depends on.
    pub fn last_import(&self) -> Option<usize> {
        let opens: &[&str] = match self.lang {
            Language::Ts => &["import ", "export * from", "export {"],
            Language::Py => &["import ", "from "],
            Language::Rs => &["use ", "extern crate "],
            Language::Go => &["import ", "package "],
        };
        self.code
            .iter()
            .enumerate()
            .filter(|(at, code)| {
                let trimmed = code.trim_start();
                *at < 80 && opens.iter().any(|open| trimmed.starts_with(open))
            })
            .map(|(at, _)| at)
            .next_back()
            .map(|at| self.statement_end(at))
    }

    /// The last line of the statement that opens on this one: itself, unless it
    /// left a bracket open.
    pub fn statement_end(&self, first: usize) -> usize {
        let mut depth = 0i32;
        for (at, code) in self.code.iter().enumerate().skip(first) {
            for character in code.chars() {
                match character {
                    '{' | '(' | '[' => depth += 1,
                    '}' | ')' | ']' => depth -= 1,
                    _ => {}
                }
            }
            if depth <= 0 {
                return at;
            }
        }
        first
    }

    /// A line inside a function that does one thing and closes everything it
    /// opened: what a handler can be wrapped around without moving anything
    /// else. A declaration is left alone, because wrapping one takes the name
    /// it binds out of the scope the rest of the body reads it in.
    pub fn simple_statement(&self) -> Option<usize> {
        let keywords: &[&str] = match self.lang {
            Language::Ts => &[
                "return", "import", "export", "const", "let", "var", "function", "class", "if",
                "for", "while", "try", "catch", "switch", "case", "default", "throw", "await", "}",
                "{", ")", "//",
            ],
            Language::Py => &[
                "return", "import", "from", "def", "class", "if", "for", "while", "try", "except",
                "else", "elif", "with", "pass", "raise", "yield", "@", "#", ")", "]",
            ],
            _ => &[],
        };
        for function in self.functions() {
            for at in function.first as usize + 1..=(function.last as usize).saturating_sub(1) {
                let code = self.code.get(at)?;
                let trimmed = code.trim();
                if trimmed.is_empty() || !balanced(code) {
                    continue;
                }
                let first = trimmed
                    .split(['(', ' ', '.', '='])
                    .next()
                    .unwrap_or_default();
                if keywords.contains(&first) {
                    continue;
                }
                let closes = match self.lang {
                    Language::Ts => trimmed.ends_with(';'),
                    Language::Py => !trimmed.ends_with(':') && !trimmed.ends_with('\\'),
                    _ => false,
                };
                if closes {
                    return Some(at);
                }
            }
        }
        None
    }
}

/// One block of a file: a test case, or a function.
#[derive(Debug, Clone)]
pub struct Case {
    pub name: String,
    /// Zero-based, the line the block opens on.
    pub first: u32,
    /// Zero-based, the line the block closes on.
    pub last: u32,
}

/// The title a call was given, where the first thing it was handed is a string.
fn title(line: &str) -> Option<String> {
    let after = line.split_once('(')?.1;
    let quote = after.chars().find(|c| matches!(c, '\'' | '"' | '`'))?;
    let (_, rest) = after.split_once(quote)?;
    let (found, _) = rest.split_once(quote)?;
    (!found.is_empty()).then(|| found.to_string())
}

fn leading_whitespace(line: &str) -> &str {
    let end = line
        .find(|c: char| !c.is_whitespace())
        .unwrap_or(line.len());
    &line[..end]
}

fn indent_of(line: &str) -> usize {
    leading_whitespace(line)
        .chars()
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

/// Whether a line closes everything it opened.
fn balanced(code: &str) -> bool {
    let mut depth = 0i32;
    for character in code.chars() {
        match character {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            _ => {}
        }
    }
    depth == 0
}

/// Every line with its strings and comments blanked out. The state that carries
/// across lines is a block comment and a string a language lets run on, which is
/// why this is done for the file rather than for a line.
fn mask(lang: Language, lines: &[String]) -> Vec<String> {
    let mut masked = Vec::with_capacity(lines.len());
    let mut carried: Option<Carried> = None;
    for line in lines {
        let (code, next) = mask_line(lang, line, carried);
        masked.push(code);
        carried = next;
    }
    masked
}

/// What a line left open for the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Carried {
    BlockComment,
    /// A string that is still open, and how it will be closed.
    Runner(Quote),
}

/// A string literal's opening, as it will have to be matched again to close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Quote {
    character: char,
    /// Python writes a string that runs on with three of its quote.
    triple: bool,
}

impl Quote {
    fn width(self) -> usize {
        if self.triple {
            3
        } else {
            1
        }
    }

    /// Whether a string opened this way may run past the end of its line.
    fn runs_on(self, lang: Language) -> bool {
        match lang {
            Language::Py => self.triple,
            Language::Go => self.character == '`',
            _ => self.character == '`',
        }
    }
}

fn mask_line(lang: Language, line: &str, carried: Option<Carried>) -> (String, Option<Carried>) {
    let characters: Vec<char> = line.chars().collect();
    let mut code: Vec<char> = Vec::with_capacity(characters.len());
    let mut state = carried;
    let mut at = 0;
    while at < characters.len() {
        let character = characters[at];
        match state {
            Some(Carried::BlockComment) => {
                if character == '*' && characters.get(at + 1) == Some(&'/') {
                    blank(&mut code, 2);
                    at += 2;
                    state = None;
                    continue;
                }
                blank(&mut code, 1);
                at += 1;
            }
            Some(Carried::Runner(quote)) => {
                if character == '\\' && quote.character != '`' {
                    blank(&mut code, 2.min(characters.len() - at));
                    at += 2;
                    continue;
                }
                if closes(&characters, at, quote) {
                    blank(&mut code, quote.width());
                    at += quote.width();
                    state = None;
                    continue;
                }
                blank(&mut code, 1);
                at += 1;
            }
            None => {
                if starts_line_comment(lang, &characters, at) {
                    blank(&mut code, characters.len() - at);
                    return (code.into_iter().collect(), None);
                }
                if lang != Language::Py && character == '/' && characters.get(at + 1) == Some(&'*')
                {
                    blank(&mut code, 2);
                    at += 2;
                    state = Some(Carried::BlockComment);
                    continue;
                }
                if let Some(quote) = opens_string(lang, &characters, at) {
                    blank(&mut code, quote.width());
                    at += quote.width();
                    state = Some(Carried::Runner(quote));
                    continue;
                }
                code.push(character);
                at += 1;
            }
        }
    }
    // A quote that cannot run on closes with the line whatever it opened, so a
    // lone apostrophe in a comment-free line never swallows the rest of a file.
    let state = match state {
        Some(Carried::Runner(quote)) if !quote.runs_on(lang) => None,
        other => other,
    };
    (code.into_iter().collect(), state)
}

fn blank(code: &mut Vec<char>, width: usize) {
    for _ in 0..width {
        code.push(' ');
    }
}

fn starts_line_comment(lang: Language, characters: &[char], at: usize) -> bool {
    match lang {
        Language::Py => characters[at] == '#',
        _ => characters[at] == '/' && characters.get(at + 1) == Some(&'/'),
    }
}

/// The string a line opens at this position, if it opens one.
fn opens_string(lang: Language, characters: &[char], at: usize) -> Option<Quote> {
    let character = characters[at];
    let quotes: &[char] = match lang {
        Language::Py => &['\'', '"'],
        Language::Go => &['"', '`', '\''],
        _ => &['"', '\'', '`'],
    };
    if !quotes.contains(&character) {
        return None;
    }
    let triple = lang == Language::Py
        && characters.get(at + 1) == Some(&character)
        && characters.get(at + 2) == Some(&character);
    Some(Quote { character, triple })
}

/// Whether the quote at this position closes the string that is open.
fn closes(characters: &[char], at: usize, quote: Quote) -> bool {
    if characters[at] != quote.character {
        return false;
    }
    if !quote.triple {
        return true;
    }
    characters.get(at + 1) == Some(&quote.character)
        && characters.get(at + 2) == Some(&quote.character)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_inside_a_string_is_not_code() {
        let source = Source::read(
            "a.test.ts",
            Language::Ts,
            "test('it.skip is a word', () => {\n  expect(1).toBe(1);\n});\n",
        );
        assert!(!source.code[0].contains("it.skip"));
        assert_eq!(source.cases().len(), 1);
        assert_eq!(source.cases()[0].last, 2);
    }

    #[test]
    fn python_cases_end_where_the_indentation_does() {
        let source = Source::read(
            "test_a.py",
            Language::Py,
            "def test_one():\n    assert 1 == 1\n\ndef test_two():\n    assert 2 == 2\n",
        );
        let cases = source.cases();
        assert_eq!(cases.len(), 2);
        assert_eq!((cases[0].first, cases[0].last), (0, 1));
        assert_eq!((cases[1].first, cases[1].last), (3, 4));
    }

    #[test]
    fn rust_cases_are_found_by_the_attribute() {
        let source = Source::read(
            "src/a.rs",
            Language::Rs,
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn one() {\n        assert!(true);\n    }\n}\n",
        );
        let cases = source.cases();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "one");
        assert!(source.holds_tests());
    }

    #[test]
    fn go_cases_are_found_by_the_prefix() {
        let source = Source::read(
            "a_test.go",
            Language::Go,
            "func TestOne(t *testing.T) {\n\tif 1 != 1 {\n\t\tt.Fatal(\"no\")\n\t}\n}\n",
        );
        assert_eq!(source.cases().len(), 1);
        assert_eq!(source.assertions(), vec![2]);
    }
}
