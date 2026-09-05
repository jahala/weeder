# Adversarial fixtures

One minimal repository history per rule per language, at
`<RULE>/<lang>/<fire|silent>/`. `before/` is committed as HEAD, `after/` replaces
the working tree, and a file `after/` does not carry is a file the change
deleted. `after/.weed-commit` is the message of the commit being prepared, for
the rules that read trailers; it never reaches the tree.

`fire/` is a change the rule must report. `silent/` is its nearest neighbour —
the change that looks like the offence and is not one. A rule with no `silent/`
fixture has not been shown to discriminate.

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
the middle of a line — the one a `silent/` fixture keeps inside a string — is
written out, because that is the case the rule has to let through.
