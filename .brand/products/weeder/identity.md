# weeder: product identity

A product layer over the plotplot umbrella. Deltas only; everything absent inherits the umbrella.

| Field | Value |
|---|---|
| Product | weeder |
| Tagline | tests still mean what they meant. |
| Accent | `#A37A6E`, the shed's deep grey, wood and iron (from the umbrella Product Accents table) |
| Faces | `weeder check` · `weeder scan` · `weeder guard` · `weeder hook` · `weeder rules` (`bite` joins when it lands) |
| Commands | `weeder check --format sarif` · `weeder guard install --protect main` |

## Positioning

The judge of the diff. Agents produce; weeder decides whether what they produced is honest
work: a test deleted or weakened, a skip added, a stub left in production code, an error
swallowed, a secret pasted in, a guardrail edited, an import crossing a boundary, a file
touched outside the agreed scope. It reads the diff and the tree with a parser, never with a
model, and answers in milliseconds with an exit code and a SARIF log.

Four faces on one core. `check` judges a diff and may block. `scan` judges the tree and never
blocks. `guard` is the law in git, at commit and push, for every agent and every human alike.
`bite` proves a test still fails without the change. It sits where the work happens: the
agent's Stop hook, pleach's gate on every node, a pre-commit hook, a check on the pull request.

Only the unambiguous shapes block. Everything else is a warning aimed at the person reading
the pull request, and every allowance a person grants is itself a visible finding.

## Mark

A pair of shears open around a weed's stem, at the height where the cut is made, drawn in ink on
a tile of weeder's bloom (`#A37A6E`): a stem with two leaves, still growing, and two crossed
blades with two open handle loops around it. The owner asked for this reading on 2026-09-05, and it is the judgement one moment
before it lands: the plant is whole, the blades are already around it, and nothing has been cut
yet. That is what `weeder check` is, a verdict delivered before the change goes in rather than a
report on damage afterwards. Drawn in the garden's diagram language, stroked stems and filled
shapes, so it sits beside umbel's inflorescence and tilth's seedling as a sibling.

Files: `assets/weeder-mark.svg` (paper) · `assets/weeder-mark-night.svg` (soil-night).

## Logo usage

- Minimum size: 16px mark height, rendered at 16 · 24 · 32 · 64 and read at each. At 16px the
  handle loops stay open and the shears-on-a-stem reading survives.
- Clear space: half the mark height on every side.
- Two values only: the tile `#A37A6E` (`#A37A6E` on soil-night) and the ink `#1C1610`.
  The night file is derived by `scripts/marks.py`, never drawn.
- The blades never touch the leaves, and the stem is never drawn already cut. The shears are
  open, always. A severed stem is a different claim from the one weeder makes.
- Never: close the handle loops into filled dots, add a hand holding them, set the blades in
  error red, or outline the wordmark.
