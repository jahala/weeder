# page — dogfood notes

Notes from working the page loop on 2026-09-06. One file per loop; the conductor folds these
into `docs/dogfood.md`.

## tend2

- The loop carried its own briefing. `## What to read before writing a line`, `## Tokens,
  colour, the mark` and `## What the page says` meant the first hour was reading four named
  files rather than guessing at a house style, and the trap list at the end of the narrative
  saved at least three rounds against the brand gate: the hex-shaped anchor, the last-wins
  `var()` resolver, and the breakpoints. A loop whose narrative is a briefing is worth more
  than a loop whose narrative is a summary.
- The `## Tried` entry said Pages is not enabled and the repository is private, so going live
  is the owner's step. That is the difference between finishing and hanging: the human check
  covers the part I cannot do, and I did not try to do it anyway.
- `tend2 verify` with `--force` re-ran both checks and stamped each with its evidence hash.
  Running it before I thought I was finished was the useful move; the first run is what told
  me the footer comparison was rejecting its own text nodes.

## The friction

- **A check can name a gate that lives in another repository, and nothing on the map says how
  to reach it.** Check c1 asks for the umbrella's `petals/scripts/check.sh` with the
  umbrella's `.brand`, fetched by the evidence itself. The only pointer to that repository is
  `.brand/products/weed/petalsrc.example`, which the shaping narrative names but the check
  text does not. `scripts/umbrella-brand.sh` now reads that file for the source, the version
  and the product, resolves the version against the source, and refuses a checkout whose
  `.brand/identity.md` declares a different one. Worth knowing for the next loop that leans on
  a sibling repository: the pointer belongs in a committed file, not in the check's prose.
- **The version in `petalsrc.example` is `v2.0.0` and the umbrella carries no tags.** The
  fetch resolves a name at the source first and falls back to the default branch, then holds
  the fetched brand to the version field it declares. That is the honest reading of a pin the
  source cannot answer by name, and it fails loudly if the umbrella's brand version moves.
- **The brand gate passes on errors alone; warnings do not fail it.** The check asks for 0 and
  0, so `scripts/check/page.sh` reads the printed counts rather than the exit code. A gate
  whose exit code is coarser than the claim needs its output read, and that is worth saying
  out loud rather than discovering after a green run.

## What is not closed, and who owns it

- **GitHub Pages is off and the repository is private.** The page is at the repository root
  and passes both code checks; publishing it is the owner's step, and the loop's human check
  is where that lands. Nothing in the tree assumes the page is live: the README carries no
  link to `jahala.github.io/weed/` yet, because until Pages is on, that link is a promise
  rather than a fact.
