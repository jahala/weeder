//! S3, a debug leftover reached production code.
//!
//! Somebody wanted to watch the program run and left the line in. Two shapes,
//! and both of them are the author talking to themselves.
//!
//! A print: a call that puts a value on a stream the library does not own. Each
//! language has its own, the console object, the built-in that writes a line,
//! the macro that shouts one, and a member whose name asks for everything a
//! value holds.
//!
//! A halt: the word that stops the program where it stands so a person can look
//! around. In shipped code nobody is there to look.
//!
//! A shell script prints for a living, so in bash a print is a leftover only
//! where it says it is one: a line labelled as debug output, `DEBUG …` or
//! `debug: …`, and the shell told to trace itself, `set -x` and its spellings,
//! which writes every command the script runs for whoever reads its output.
//!
//! Three places are allowed to do both. A test says what it saw. The entry
//! point is the program talking to whoever started it. And a module the
//! repository's own `[entrypoints] cli` names is the command line itself, where
//! printing is the product. The last of those is the repository's statement,
//! not a folder name weeder has heard of.

use std::ops::RangeInclusive;

use crate::core::change::Change;
use crate::core::classify::{FileKind, Lang};
use crate::core::finding::{Detection, Message, Region, Stamp};
use crate::core::glob;
use crate::core::rules::check::vocab::{holds_word, names, Name};
use crate::core::rules::check::Judgement;
use crate::core::shell;
use crate::core::syntax::{words, Mask};

/// The name a program's entry point carries in every language weeder reads.
const ENTRY: &str = "main";

/// A receiver that writes to a stream, and the members of it that do. A
/// receiver named with no members writes with every member it has.
struct Printer {
    receiver: &'static str,
    /// The word a member's name starts with when it writes, read without case.
    members: &'static [&'static str],
}

/// What a language writes when somebody is watching it run.
struct Debugging {
    /// Objects and packages whose members put a value on a stream.
    printers: &'static [Printer],
    /// Calls and macros that write a line on their own, with no receiver in
    /// front of them.
    shouts: &'static [&'static str],
    /// Words that stop the program so a person can look at it.
    halts: &'static [&'static str],
    /// Words that make a called member a request to see everything a value
    /// holds, whichever library spells it.
    inspects: &'static [&'static str],
    /// The shell's options that make it print every command it runs, as `set`
    /// spells them: a letter after `-`, and a name after `-o`.
    trace_flags: &'static [char],
    trace_options: &'static [&'static str],
    /// Commands that print, which print a debug trace where what they print
    /// is labelled as one.
    labelled_prints: &'static [&'static str],
}

const NOTHING_WATCHED: Debugging = Debugging {
    printers: &[],
    shouts: &[],
    halts: &[],
    inspects: &[],
    trace_flags: &[],
    trace_options: &[],
    labelled_prints: &[],
};

/// The command that sets the shell's options.
const SET: &str = "set";

/// The word a line of output is labelled with when it was written for whoever
/// was watching the program run.
const DEBUG_LABEL: &str = "debug";

fn debugging(lang: Lang) -> Debugging {
    match lang {
        // The console object, and the statement that hands the program to
        // whatever is attached to it. The members that report a failure are
        // left out: those are how a library speaks up, not how it is watched.
        Lang::TypeScript | Lang::JavaScript => Debugging {
            printers: &[Printer {
                receiver: "console",
                members: &["log", "debug", "trace", "dir", "table"],
            }],
            halts: &["debugger"],
            ..NOTHING_WATCHED
        },
        // The built-in that writes a line, the built-in that stops, and the
        // standard library's debugger, which is reached as a member.
        Lang::Python => Debugging {
            shouts: &["print"],
            halts: &["breakpoint", "set_trace"],
            ..NOTHING_WATCHED
        },
        // The macros that write a line, and the one that shouts a value back
        // with the expression that produced it.
        Lang::Rust => Debugging {
            shouts: &["print", "println", "dbg"],
            ..NOTHING_WATCHED
        },
        // The formatting package's printing members, which are the ones whose
        // names begin with the word, and the built-ins that do the same. What
        // that package formats rather than prints is left alone.
        Lang::Go => Debugging {
            printers: &[Printer {
                receiver: "fmt",
                members: &["print"],
            }],
            shouts: &["print", "println"],
            inspects: &["dump"],
            ..NOTHING_WATCHED
        },
        // Tracing, `-x`, writes each command before it runs, and verbose
        // mode, `-v`, each line as it is read. The printing commands are the
        // script's product, and only a line labelled as debug output is not.
        Lang::Bash => Debugging {
            trace_flags: &['x', 'v'],
            trace_options: &["xtrace", "verbose"],
            labelled_prints: &["echo", "printf"],
            ..NOTHING_WATCHED
        },
        Lang::Other => NOTHING_WATCHED,
    }
}

pub fn evaluate(judged: &Judgement) -> Vec<Detection> {
    let mut findings = Vec::new();
    for change in judged.changes {
        let Some(path) = change.diff.new_path.as_deref() else {
            continue;
        };
        if !change.after.is(FileKind::Prod) || is_entrypoint(path, judged) {
            continue;
        }
        let lang = lang(change);
        let mask = change.after.mask();
        let table = debugging(lang);
        let entry = entry_point(change);
        for (line, _) in change.added() {
            if entry.iter().any(|span| span.contains(&line)) {
                continue;
            }
            if let Some(leftover) = leftover(&table, mask, line) {
                findings.push(finding(path, line, &leftover));
            }
        }
    }
    findings
}

/// What an added line left behind, or nothing where it left nothing.
fn leftover(table: &Debugging, mask: &Mask, line: u32) -> Option<Leftover> {
    if let Some(found) = shell_leftover(table, mask, line) {
        return Some(found);
    }
    let code = mask.code(line);
    if let Some(halt) = words(&code)
        .into_iter()
        .find(|word| table.halts.contains(&word.text))
    {
        return Some(Leftover::halt(halt.text));
    }
    let printed = names(&code).into_iter().find(|name| prints(table, name))?;
    Some(Leftover::print(printed.text))
}

/// What an added line of a shell script left behind: a command that turns
/// tracing on, or one that prints a line labelled as debug output. The line is
/// read as the commands it runs, with its comments taken out, and a command
/// counts only where its name is code, so a line inside a string or a heredoc
/// runs nothing.
fn shell_leftover(table: &Debugging, mask: &Mask, line: u32) -> Option<Leftover> {
    if table.trace_flags.is_empty() && table.labelled_prints.is_empty() {
        return None;
    }
    let code = mask.code(line);
    let spoken: Vec<&str> = words(&code).into_iter().map(|word| word.text).collect();
    for command in shell::commands(&mask.outside_comments(line)) {
        let (command, _) = shell::simple(&command);
        let Some((name, arguments)) = command.split_first() else {
            continue;
        };
        if !spoken.contains(&name.as_str()) {
            continue;
        }
        if name == SET && traces(table, arguments) {
            return Some(Leftover::trace(&command.join(" ")));
        }
        if table.labelled_prints.contains(&name.as_str()) && labelled(arguments) {
            return Some(Leftover::labelled(name));
        }
    }
    None
}

/// Whether `set` is given an option that turns tracing on: a letter in a run
/// of them after `-`, `-euxo`, or a name after `-o`. `+x` turns it off.
fn traces(table: &Debugging, arguments: &[String]) -> bool {
    arguments.iter().enumerate().any(|(at, argument)| {
        let Some(flags) = argument.strip_prefix('-') else {
            return false;
        };
        let named = flags.ends_with('o')
            && arguments
                .get(at + 1)
                .is_some_and(|option| table.trace_options.contains(&option.as_str()));
        flags.chars().all(|flag| flag.is_ascii_alphabetic())
            && (named || flags.chars().any(|flag| table.trace_flags.contains(&flag)))
    })
}

/// Whether what a printing command prints is labelled as debug output: the
/// first thing it prints, past its own options, opens with the label shouted,
/// `DEBUG`, or written as a label, `debug:`, `[debug]`. A sentence that begins
/// with the word, `Debug builds go to …`, is output like any other.
fn labelled(arguments: &[String]) -> bool {
    let Some(printed) = arguments.iter().find(|argument| !argument.starts_with('-')) else {
        return false;
    };
    let opened = printed.trim_start_matches(['[', '(', '<', ' ']);
    let word: String = opened
        .chars()
        .take_while(|character| character.is_alphabetic())
        .collect();
    if !word.eq_ignore_ascii_case(DEBUG_LABEL) {
        return false;
    }
    let after = opened[word.len()..].chars().next();
    word == DEBUG_LABEL.to_ascii_uppercase() || matches!(after, Some(':' | ']' | ')' | '>' | '='))
}

/// Whether a name is a call that puts a value somewhere a person can read it.
fn prints(table: &Debugging, name: &Name<'_>) -> bool {
    if !name.is_called() && !name.is_macro() {
        return false;
    }
    let last = name.last().to_ascii_lowercase();
    if let Some(receiver) = name.receiver() {
        let written = table.printers.iter().any(|printer| {
            printer.receiver.eq_ignore_ascii_case(receiver)
                && (printer.members.is_empty()
                    || printer.members.iter().any(|word| last.starts_with(word)))
        });
        return written || holds_word(name.last(), table.inspects);
    }
    // A shout is the language's own word, so it counts where nothing was
    // reached through: a member named `print` on an object is that object's
    // business, and its own library's to judge.
    table.shouts.contains(&name.last())
}

/// Whether the repository named this file as the command line itself.
fn is_entrypoint(path: &str, judged: &Judgement) -> bool {
    glob::matches_any(&judged.config.cli_entrypoints, path)
        || path.rsplit('/').next().unwrap_or(path).starts_with("main.")
}

/// The lines the function the program starts at occupies. It is read once per
/// file: the outline of a large file is large, and asking it about every line
/// would make the cost of a check the square of the size of the change.
fn entry_point(change: &Change) -> Vec<RangeInclusive<u32>> {
    change
        .after
        .outline
        .flatten()
        .iter()
        .filter(|definition| definition.name == ENTRY)
        .map(|definition| definition.start_line..=definition.end_line)
        .collect()
}

fn lang(change: &Change) -> Lang {
    change
        .after
        .classification
        .as_ref()
        .map_or(Lang::Other, |classification| classification.lang)
}

/// A leftover, as it will be quoted back to the reader.
struct Leftover {
    what: String,
    why: &'static str,
    next: &'static str,
}

impl Leftover {
    fn print(call: &str) -> Leftover {
        Leftover {
            what: format!("production code was added that prints for a person to read: `{call}`."),
            why: "a library writing to a stream it does not own is output the caller never asked for.",
            next: "take the line out, or report it through whatever this repository logs with.",
        }
    }

    fn trace(command: &str) -> Leftover {
        Leftover {
            what: format!("production code was added that turns tracing on: `{command}`."),
            why: "the shell writes every command it runs, and what it ran them with, to the output of whoever runs the script.",
            next: "take the line out, and trace from outside with `bash -x` when someone needs to watch the script run.",
        }
    }

    fn labelled(command: &str) -> Leftover {
        Leftover {
            what: format!("production code was added that prints a line labelled as debug output, with `{command}`."),
            why: "the label says the line was written for whoever was watching the script run, and whoever runs it now reads it as output.",
            next: "take the line out, or print it only when the script is asked to say more.",
        }
    }

    fn halt(word: &str) -> Leftover {
        Leftover {
            what: format!("production code was added that stops for a debugger: `{word}`."),
            why: "the program waits for somebody to look at it, and in a running system nobody is there.",
            next: "take the line out before the change lands.",
        }
    }
}

fn finding(path: &str, line: u32, leftover: &Leftover) -> Detection {
    Detection {
        stamp: Stamp::Rule,
        path: path.to_string(),
        region: Some(Region {
            start_line: line,
            end_line: line,
        }),
        message: Message {
            what: leftover.what.clone(),
            why: leftover.why.to_string(),
            next: leftover.next.to_string(),
        },
        fix: None,
    }
}
