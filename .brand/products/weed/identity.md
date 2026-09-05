# weed — product identity

A product layer over the plotplot umbrella. Deltas only; everything absent inherits the umbrella.

| Field | Value |
|---|---|
| Product | weed |
| Tagline | tests still mean what they meant. |
| Accent | #8E3B5E bramble — the weed that stops you (from the umbrella Product Accents table) |
| Faces | `weed check` · `weed guard` · `weed hook` · `weed rules` — `scan` and `bite` join when they land |
| Commands | `weed check --format sarif` · `weed guard install --protect main` |

## Positioning

The judge of the diff. Agents produce; weed decides whether what they produced is honest
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

A pair of shears open around a weed's stem, at the height where the cut is made. The plant is
growth green (`#357E2C`): a stem, three leaves, still growing. The shears are bramble
(`#8E3B5E`): two crossed blades and two open handle loops, the only thing in the mark that is
not alive. The owner asked for this reading on 2026-09-05, and it is the judgement one moment
before it lands: the plant is whole, the blades are already around it, and nothing has been cut
yet. That is what `weed check` is, a verdict delivered before the change goes in rather than a
report on damage afterwards. Drawn in the garden's diagram language, stroked stems and filled
accent shapes, so it sits beside umbel's inflorescence and pollen's two leaning stalks as a
sibling.

Files: `assets/weed-mark.svg` (paper) · `assets/weed-mark-night.svg` (soil-night).

## Logo usage

- Minimum size: 20px mark height, rendered at 16 · 20 · 28 · 48 and read at each. At 20px the
  leaves flatten and the handle loops close up, and the shears-on-a-stem reading survives. At
  16px it does not; use the wordmark below 20.
- Clear space: half the mark height on every side.
- Fills are exact — growth green `#357E2C` and bramble `#8E3B5E` on paper; on soil-night the
  plant brightens to `#84C56A` and the shears stay bramble `#8E3B5E` as a fill, while any bramble
  **word** on night lifts to `#B85C82` (the umbrella's night rule). Never recolour outside those
  four values.
- The blades never touch the leaves, and the stem is never drawn already cut. The shears are
  open, always. A severed stem is a different claim from the one weed makes.
- Never: colour the plant in bramble, colour the shears green, close the handle loops into
  filled dots, add a hand holding them, set the blades in error red, or outline the wordmark.
