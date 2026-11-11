//! The syntax mask, reading bash (#79).
//!
//! A rule that looks for a token in a shell script has to know the script's
//! comments and strings the way the shell does. `#` opens a comment only where
//! a word begins, so `$#`, `${#name}` and `a#b` are code. A single quote runs
//! to the next one, across lines and with no escapes; `$'…'` is the one that
//! reads escapes; a double quote reads them too and may hold a command
//! substitution with quotes of its own. A heredoc's body is literal from the
//! line after the one that opens it to the line that is its delimiter.

use weeder::core::syntax::Mask;
use weeder::core::Lang;

fn mask(source: &str) -> Mask {
    Mask::of(Lang::Bash, source)
}

fn trimmed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

#[test]
fn a_hash_opens_a_comment_only_where_a_word_begins() {
    let read = mask(
        "# the whole line\n\
         echo hi # a note\n\
         echo $# ${#name} ${path#*/} a#b\n\
         run;# after a semicolon\n",
    );
    assert_eq!(read.code(1).trim(), "");
    assert_eq!(read.comments(1).trim(), "# the whole line");
    assert_eq!(read.code(2).trim(), "echo hi");
    assert_eq!(read.comments(2).trim(), "# a note");
    assert_eq!(
        read.code(3).trim(),
        "echo $# ${#name} ${path#*/} a#b",
        "a hash inside a word or an expansion is code"
    );
    assert_eq!(read.comments(3).trim(), "");
    assert_eq!(read.comments(4).trim(), "# after a semicolon");
}

#[test]
fn quotes_hold_a_hash_and_a_single_quote_reads_no_escapes() {
    let read = mask("echo \"a # b\" 'c # d' $'e\\'s # f' done\n");
    assert_eq!(read.comments(1).trim(), "", "every hash here is quoted");
    let literals: Vec<String> = read
        .literal_runs(1)
        .into_iter()
        .map(|literal| literal.text)
        .collect();
    assert_eq!(
        literals,
        vec!["\"a # b\"", "'c # d'", "$'e\\'s # f'"],
        "a `$'` string reads its escape and ends at the quote after it"
    );
    assert_eq!(trimmed(&read.code(1)), "echo done");
}

#[test]
fn a_single_quoted_string_runs_across_lines_and_an_escaped_quote_opens_none() {
    let read = mask(
        "message='first line\n\
         # still the string'\n\
         echo It\\'s here # a note\n",
    );
    assert_eq!(
        read.code(2).trim(),
        "",
        "the second line is inside the string"
    );
    assert_eq!(read.comments(2).trim(), "");
    assert_eq!(read.literals(2).trim(), "# still the string'");
    assert_eq!(
        read.comments(3).trim(),
        "# a note",
        "a backslash before a quote in code opens no string"
    );
}

#[test]
fn a_command_substitution_inside_double_quotes_keeps_its_own_quotes() {
    let read = mask(
        "local said=\"$(printf '%s' \"it's\") and more\"\n\
         # a comment after it\n",
    );
    assert_eq!(
        read.comments(2).trim(),
        "# a comment after it",
        "the apostrophe inside the nested string opens nothing on the line after"
    );
    assert!(read.code(1).contains("printf"), "{}", read.code(1));
    assert!(read.literals(1).contains("it's"), "{}", read.literals(1));
}

#[test]
fn a_heredoc_body_is_literal_up_to_its_delimiter_line() {
    let read = mask(
        "cat <<EOF\n\
         # not a comment, $HOME\n\
         it's a body\n\
         EOF\n\
         echo after # a note\n",
    );
    assert_eq!(read.code(1).trim(), "cat <<EOF");
    for line in 2..=4 {
        assert_eq!(read.code(line).trim(), "", "line {line} is the heredoc");
        assert_eq!(read.comments(line).trim(), "", "line {line} is the heredoc");
    }
    assert_eq!(read.code(5).trim(), "echo after");
    assert_eq!(read.comments(5).trim(), "# a note");
}

#[test]
fn a_dashed_heredoc_ends_at_a_tab_indented_delimiter_and_a_quoted_one_reads_its_word() {
    let read = mask(
        "if true; then\n\
         \tcat <<-'END'\n\
         \t# literal, and so is $x\n\
         \tEND\n\
         \techo after # a note\n\
         fi\n",
    );
    assert_eq!(read.code(3).trim(), "");
    assert_eq!(
        read.code(4).trim(),
        "",
        "the delimiter line belongs to the heredoc"
    );
    assert_eq!(read.code(5).trim(), "echo after");
    assert_eq!(read.comments(5).trim(), "# a note");
    assert_eq!(read.code(6).trim(), "fi");
}

#[test]
fn a_here_string_and_a_shift_open_no_heredoc() {
    let read = mask(
        "read -r name <<< \"$line\"\n\
         (( mask = 1 << bits ))\n\
         echo next # a note\n",
    );
    assert_eq!(read.code(3).trim(), "echo next");
    assert_eq!(read.comments(3).trim(), "# a note");
}

#[test]
fn two_heredocs_on_one_line_are_read_one_after_the_other() {
    let read = mask(
        "paste <(cat <<A\n\
         one\n\
         A\n\
         ) <(cat <<B\n\
         two\n\
         B\n\
         )\n\
         echo done # a note\n",
    );
    assert_eq!(read.code(2).trim(), "");
    assert_eq!(read.code(4).trim(), ") <(cat <<B");
    assert_eq!(read.code(5).trim(), "");
    assert_eq!(read.code(8).trim(), "echo done");
    assert_eq!(read.comments(8).trim(), "# a note");
}
