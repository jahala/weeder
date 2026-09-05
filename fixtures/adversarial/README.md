# Adversarial fixtures

One minimal repository history per rule per language, at
`<RULE>/<lang>/<fire|silent>/`. `before/` is committed as HEAD, `after/` replaces
the working tree, and a file `after/` does not carry is a file the change
deleted. `after/.weed-commit` is the message of the commit being prepared, for
the rules that read trailers; it never reaches the tree.

`fire/` is a change the rule must report. `silent/` is its nearest neighbour , 
the change that looks like the offence and is not one. A rule with no `silent/`
fixture has not been shown to discriminate.

A rule with more than one shape to prove carries more than one of each, named
for what it holds: T5 has a `silent-code/` for the half where the code moves
without the expectation, and T7 has `fire-case/` and `silent-case/` for the
languages whose runners collect a case by the name it is declared under. Every
directory holding a `before/` and an `after/` is a fixture, whatever it is
called, and `tests/determinism.rs` replays all of them.

A rule on the `scan` face judges the tree rather than a diff, so its fixture has
one state and not two: `before/` alone, committed as HEAD and left in place as
the working tree. `after/` is absent, and the harness knows to stop there.

`tests/common/mod.rs` builds the repository. Nothing here is copied by hand.

## Conflict markers

weed does not carry what it refuses, so no file here opens a line with a
conflict marker. A fixture writes these placeholders instead, and the harness
expands each into seven of its character before the file reaches the temp
repository:

| placeholder            | marker    |
| ---------------------- | --------- |
| `{{weed:ours}}`        | the opener |
| `{{weed:base}}`        | the base side of a diff3 merge |
| `{{weed:separator}}`   | the divider |
| `{{weed:theirs}}`      | the closer |

A label follows the placeholder as it follows the marker, so
`{{weed:ours}} HEAD` becomes the line git wrote. The binary under test reads the
real bytes; only the file on disk here is spelled differently. A separator in
the middle of a line, the one a `silent/` fixture keeps inside a string, is
written out, because that is the case the rule has to let through.
