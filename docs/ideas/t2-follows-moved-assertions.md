# T2 should follow an assertion into the file it moved to

The calibration ledger, redone with the classes read off the finding's own
sentence, put seven blocked commits in the false-positive column. Three of them
are one shape, and it is T2's:

- tilth `18eb6643ff`, "tighten module visibility and co-locate tests": 21
  assertions left `src/mcp/mod.rs` and arrived in `src/mcp/tools/mod.rs`,
  `files.rs` and `edit.rs` in the same commit.
- tilth `5a4edbf5c5`, "extract bloom_walk, callee_query, scope": 21 assertions
  left `src/search/callers.rs` and 26 arrived in the three new modules the same
  commit creates.
- tilth `11aef933c9` is the T1 version of it that the sharpened T1 no longer
  reports, and the commit still blocks on nothing else.

T2 says more than a count. It says "a case that checks nothing passes whatever
the code does, so the suite reports green over behaviour nobody is holding", and
where the same change puts those checks in another file that sentence is untrue.
T1 was sharpened for exactly this on rules-tests: it follows a case into the file
it moved to, by the name its runner collects it under and by what it holds with
that name taken out. T2 counts per file and follows nothing.

The blind re-grade read the same three commits and called them reorganisation
rather than weakening, which is the auditor saying the same thing in the other
direction.

Cost of leaving it: a refactor that splits a test file blocks, and the class of
change weed most wants people to keep making is the one it makes most expensive.
Cost of fixing it: T1's arrival machinery already exists in
`src/core/rules/check/t1.rs`; T2 needs the assertions a file lost looked for in
the rest of the change before it reports them, and a finding that names the files
they moved into where some of them did.

This attaches to rules-tests rather than standing alone. It is parked here
because a calibration worker cannot write on another loop's page.
