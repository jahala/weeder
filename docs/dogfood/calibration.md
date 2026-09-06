# calibration — dogfood notes

## tend2

The loop file carried the whole job. Its narrative decided three things the
checks alone would have left me guessing at: that the repositories are named
with paths, that the bar is pooled rather than per repository, and that
classification is a person's job and the script only checks the arithmetic.
That last line is the one that shaped the design most: it is why the
classifications live in `docs/calibration/judgements.toml` and the report is
generated from measurement plus ledger, rather than being a document somebody
edits and a script then believes.

`## Tried` earned its keep. Four of its five lines changed what I built, and the
fifth, cape-town's ruling on specimen exclusions, is the one thing on this loop
I did not land. It is not one of the four checks, it changes a detector's config
surface, and it would have moved the numbers this loop exists to record. It is
named here rather than left in the dark.

`tend2 verify --force --expect-payload` is a good gate to work against: the
payload hash means I could not have quietly reworded the goal to fit what I
built.

## pleach

The work order was complete enough to start from cold. The one thing it does
not say, and could, is what the worker may change outside its own files. This
loop needed a `.cargo/config.toml` for the `cargo xtask` alias, and
`scripts/check/tilth-core-dep.sh` refused any cargo config beside the manifest.
Tightening that check from "no config" to "no table but `[alias]`, and no
`paths` key" keeps the guarantee it was written for and lets the alias exist,
but it changes another loop's evidence file, so **tilth-core c2 needs its stamp
re-earned after this lands**. A work order that named the evidence files a node
may touch would have let me raise that before writing code rather than after.

## weed, on itself

Judging 635 real commits found three defects worth naming, and they are in the
report under `Where weed was wrong`:

- **X1** reads a `name = value` shape wherever it finds one, so a css class
  attribute (`class="detail-meta__key">`) and a line of markdown prose about
  credentials both read as an assigned secret. Seven of this run's nine false
  positives are that one shape, across four repositories. It is the single
  highest-value fix on the rule side.
- **S1** calls a Python `Protocol` method with an ellipsis body a stub. In
  Python that ellipsis is how a protocol declares a method's type; there is no
  implementation to finish.
- **T1** counts statically declared test cases, so a case generated inside a
  `for` loop reads as a case that disappeared.

None of them was fixed here. Calibration measures; a rule that changes while it
is being measured measures nothing, and each of these belongs to the loop that
owns its rule. The bar is met without them: 1.42 percent pooled, and 0.31
percent if X1 alone were fixed.

The friction the gate creates is almost all C1: 42 of 71 blocks are a GitHub
workflow edit, every one of them real. Whether that friction is wanted is a
product decision this loop can only put a number on.
