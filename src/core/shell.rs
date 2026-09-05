//! A command line read the way a shell reads one.
//!
//! weed runs nothing here. A harness hands its hook the text of a command an
//! agent is about to run, and weed has to know whether that text asks git for a
//! commit — behind an `&&`, down a pipe, inside a `bash -lc` a wrapper built, or
//! nowhere at all because the words only sit inside an `echo`. Quoting is what
//! makes that question hard, so quoting is what this module does: the words a
//! shell would build, grouped into the commands it would run.
//!
//! It is deliberately not a shell. Substitution, expansion and control flow are
//! left alone: `$(git commit)` reads as one word and is not opened, and a `for`
//! loop's body reads as the commands it holds. What matters is that a word only
//! becomes a command's name where the shell would run it as one.

/// How deep a `sh -c` inside a `sh -c` is followed. Deeper than this is nothing
/// a person wrote, and weed stops rather than recurses forever.
const MAX_DEPTH: usize = 8;

/// The programs that take a script as an argument and run it as a command line.
const SHELLS: [&str; 6] = ["sh", "bash", "zsh", "dash", "ksh", "ash"];

/// Every command a line runs, in the order it runs them, each broken into the
/// words a shell would hand the program. A shell called with a script takes the
/// place of the script's own commands, because what will run is what weed is
/// being asked about.
pub fn commands(line: &str) -> Vec<Vec<String>> {
    opened(grouped(&tokens(line)), 0)
}

/// The same, for a command a harness already broke into words. Codex hands a
/// shell call its argv rather than a line, and `["bash", "-lc", "git commit"]`
/// is the same question as `bash -lc "git commit"`.
pub fn commands_of(words: &[String]) -> Vec<Vec<String>> {
    let tokens: Vec<Token> = words.iter().cloned().map(Token::Word).collect();
    opened(grouped(&tokens), 0)
}

/// The name a command is run under, without the path it was found at.
pub fn program(command: &[String]) -> &str {
    let Some(first) = command.first() else {
        return "";
    };
    match first.rsplit_once('/') {
        Some((_, name)) => name,
        None => first.as_str(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    /// Where one command ends and the next begins: an operator, or a newline.
    Break,
}

/// The words and the breaks between them, with quoting taken off and the parts
/// a shell would not hand the program — redirections, here-document bodies,
/// comments — left out.
fn tokens(line: &str) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    // The delimiters of here-documents opened on the line being read, in the
    // order their bodies will arrive.
    let mut pending: Vec<String> = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        let character = chars[index];
        if character == '\n' {
            index += 1;
            tokens.push(Token::Break);
            index = past_bodies(&chars, index, &mut pending);
            continue;
        }
        if character.is_whitespace() {
            index += 1;
            continue;
        }
        if character == '#' {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if matches!(character, ';' | '&' | '|' | '(' | ')') {
            index += 1;
            tokens.push(Token::Break);
            continue;
        }
        if matches!(character, '<' | '>') {
            index = past_redirection(&chars, index, &mut pending);
            continue;
        }
        let Some(word) = read_word(&chars, &mut index) else {
            // Nothing readable here, and nothing that moves the cursor either.
            index += 1;
            continue;
        };
        // A bare number before a redirection is the file descriptor it applies
        // to, not a word the program will see.
        if matches!(chars.get(index), Some('<' | '>'))
            && !word.is_empty()
            && word.chars().all(|character| character.is_ascii_digit())
        {
            continue;
        }
        tokens.push(Token::Word(word));
    }
    tokens
}

/// One word, with quoting taken off, from `index`. `None` where there is no word
/// at `index` at all.
fn read_word(chars: &[char], index: &mut usize) -> Option<String> {
    let mut word = String::new();
    let mut found = false;
    while let Some(&character) = chars.get(*index) {
        match character {
            '\'' => {
                found = true;
                *index += 1;
                while let Some(&quoted) = chars.get(*index) {
                    *index += 1;
                    if quoted == '\'' {
                        break;
                    }
                    word.push(quoted);
                }
            }
            '"' => {
                found = true;
                *index += 1;
                while let Some(&quoted) = chars.get(*index) {
                    *index += 1;
                    if quoted == '"' {
                        break;
                    }
                    // Inside double quotes a backslash escapes only these four;
                    // anywhere else it stays a backslash.
                    if quoted == '\\' {
                        match chars.get(*index) {
                            Some(&next) if matches!(next, '"' | '\\' | '$' | '`') => {
                                word.push(next);
                                *index += 1;
                            }
                            Some('\n') => *index += 1,
                            _ => word.push('\\'),
                        }
                        continue;
                    }
                    word.push(quoted);
                }
            }
            '\\' => {
                found = true;
                *index += 1;
                if let Some(&next) = chars.get(*index) {
                    *index += 1;
                    // A backslash before a newline joins the two lines and
                    // leaves nothing behind.
                    if next != '\n' {
                        word.push(next);
                    }
                }
            }
            ';' | '&' | '|' | '(' | ')' | '<' | '>' => break,
            character if character.is_whitespace() => break,
            character => {
                found = true;
                word.push(character);
                *index += 1;
            }
        }
    }
    if found {
        Some(word)
    } else {
        None
    }
}

/// Past a redirection and whatever it redirects to. A here-document leaves its
/// delimiter behind, so the body can be skipped when the line ends.
fn past_redirection(chars: &[char], mut index: usize, pending: &mut Vec<String>) -> usize {
    let here_document = chars[index] == '<'
        && chars.get(index + 1) == Some(&'<')
        // `<<<` is a here-string: one word, not a body.
        && chars.get(index + 2) != Some(&'<');
    index += 1;
    if here_document {
        index += 1;
        if chars.get(index) == Some(&'-') {
            index += 1;
        }
    } else {
        while matches!(chars.get(index), Some('<' | '>' | '&')) {
            index += 1;
        }
    }
    while matches!(chars.get(index), Some(character) if character.is_whitespace() && *character != '\n')
    {
        index += 1;
    }
    let target = read_word(chars, &mut index);
    if here_document {
        pending.push(target.unwrap_or_default());
    }
    index
}

/// Past the bodies of the here-documents opened on the line just read. What is
/// in them is data the shell hands a program on its stdin, not commands.
fn past_bodies(chars: &[char], mut index: usize, pending: &mut Vec<String>) -> usize {
    for delimiter in pending.drain(..) {
        while index < chars.len() {
            let start = index;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            let line: String = chars[start..index].iter().collect();
            if index < chars.len() {
                index += 1;
            }
            if line.trim() == delimiter {
                break;
            }
        }
    }
    index
}

/// The tokens gathered into commands, with the environment a command is given
/// for its own run left out: it comes before the program's name and is not part
/// of what the program is asked to do.
fn grouped(tokens: &[Token]) -> Vec<Vec<String>> {
    let mut commands = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for token in tokens {
        match token {
            Token::Break => {
                if !current.is_empty() {
                    commands.push(std::mem::take(&mut current));
                }
            }
            Token::Word(word) => {
                if current.is_empty() && is_assignment(word) {
                    continue;
                }
                current.push(word.clone());
            }
        }
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
}

/// A shell called with a script, replaced by the commands in that script.
fn opened(commands: Vec<Vec<String>>, depth: usize) -> Vec<Vec<String>> {
    let mut opened_commands = Vec::new();
    for command in commands {
        match script(&command) {
            Some(script) if depth < MAX_DEPTH => {
                opened_commands.extend(self::opened(grouped(&tokens(&script)), depth + 1));
            }
            _ => opened_commands.push(command),
        }
    }
    opened_commands
}

/// The script a shell was given to run, where this command is a shell being
/// given one.
fn script(command: &[String]) -> Option<String> {
    if !SHELLS.contains(&program(command)) {
        return None;
    }
    for (position, word) in command.iter().enumerate().skip(1) {
        if word == "--" {
            return None;
        }
        if !word.starts_with('-') || word.len() < 2 {
            // The first word that is not an option is the script's name on
            // disk, which weed cannot read from here.
            return None;
        }
        if word[1..].contains('c') {
            return command.get(position + 1).cloned();
        }
    }
    None
}

/// Whether a word sets an environment variable for the command that follows it.
fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}
