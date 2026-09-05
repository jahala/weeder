# weed — colour delta

Inherits the full plotplot palette. One product claim:

- **Accent — bramble `#8E3B5E`** (umbrella Product Accents table). weed uses it for its mark,
  the bloom dot in the garden row, rule ids and the `[weed]` prefix, and the block-level
  marker in its table output. It **reads as a word on paper** (6.6:1), so weed needs no
  second ink colour there: kickers, labels and inline product mentions set in bramble are
  legible as they are.
- **On soil-night** bramble stays `#8E3B5E` as a fill and a dot (2.5:1, display class, the
  same as sunlight on paper). Any bramble **word** on night lifts to **bramble-night
  `#B85C82`** (4.2:1 on `#1C1610`, labels class). bramble-night sits 18 ΔE from petal
  `#E588A0`; never set the two as adjacent text on the dark theme.
- Status stays umbrella: **healthy `#46913C`**, caution `#B0741C`, **error `#BC4126`**.
  Bramble marks *weed*; error marks a *block-level result*; caution marks a *warning*; healthy
  marks *clean*. Never set a finding in bramble to make it look serious, and never set the
  thorns in error red.

Measured 2026-09-05: bramble on paper 6.57, on night 2.51, on forest 1.41 (the footer dot is
decorative); nearest claimed blooms plum 26 ΔE and petal 30 ΔE at 38° and 38° of hue.

No other colours are added.

## Measured

The umbrella's method (`scripts/palette_contrast.py`, WCAG 2.x), recomputed by `scripts/check/brand.sh` every time this layer is verified, so a number here is never older than the check.

| Colour | Hex | on Paper #FAF5E9 | on Band #F2EAD6 | with Ink #3A2718 | on Soil-night #1C1610 |
|---|---|---|---|---|---|
| bramble | #8E3B5E | 6.57 | 5.96 | 1.98 | 2.51 |
| bramble-night | #B85C82 | 3.95 | 3.58 | 3.30 | 4.17 |
| plant on night | #84C56A | 1.90 | 1.72 | 6.86 | 8.69 |
