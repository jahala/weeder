# weed — colour delta

Inherits the full plotplot palette. One claim is open: the accent.

Seven accents are taken — tilth sky `#4E88A6`, tend poppy `#D6502F`, petals petal `#E588A0`,
pleach plum `#97539B`, umbel sunlight `#E89227`, copeca juniper `#1F8A7B`, pollen anther gold
`#C8B330`. weed needs an eighth, far enough in hue from all of them to be told apart in a row
of product marks.

## Accent candidates **[flagged]**

Three proposals, measured with the umbrella's method (`scripts/palette_contrast.py`, WCAG 2.x)
against the four surfaces a plotplot product lands on. Classes: **reading** ≥ 4.5:1 ·
**labels & display** ≥ 3.0:1 · **decorative** exempt. The owner picks one and writes the row in
the umbrella's Product Accents table; until then no page, mark or terminal string treats any of
them as settled.

| Candidate | Hex | on Paper #FAF5E9 | on Band #F2EAD6 | with Ink #3A2718 | on Soil-night #1C1610 | Class on paper |
|---|---|---|---|---|---|---|
| bramble | #8E3B5E | 6.6 | 6.0 | 2.0 | 2.5 | reading |
| woad | #3D4E8C | 7.2 | 6.6 | 1.8 | 2.3 | reading |
| nettle | #6F8F1F | 3.4 | 3.1 | 3.8 | 4.8 | labels |

**bramble `#8E3B5E`** is the proposal. A bramble is the weed that stops you, which is the
product in one word, and the hue sits between petals' pink and pleach's plum without being
either — dark where both are light. It reads as a word on paper at 6.6:1, so a finding's rule
id can be set in it without a second ink colour. Paper reads on it at the same 6.6:1, which is
the fill a block-level count sits in.

**woad `#3D4E8C`** is the safe one: a deep indigo, the highest paper contrast of the three, and
no bloom in the garden is near it. It reads as institutional rather than as a weed, and it is
one hue step from tilth's sky in a row of marks.

**nettle `#6F8F1F`** is the most literal — a weed everyone has been stung by — and the worst
behaved. At 3.4:1 it is display and fill only on paper, so it needs a word form the way pollen
needs pollen-ink, and it sits close enough to the umbrella's growth green to read as a duller
version of the primary rather than as its own accent.

## The night variants, and the word form nettle needs

| Colour | Hex | on Paper #FAF5E9 | on Band #F2EAD6 | with Ink #3A2718 | on Soil-night #1C1610 | Class on soil-night |
|---|---|---|---|---|---|---|
| bramble-night | #D9799B | 2.7 | 2.4 | 4.8 | 6.1 | reading |
| woad-night | #7E93D6 | 2.7 | 2.5 | 4.7 | 6.0 | reading |
| nettle-night | #A8C846 | 1.8 | 1.6 | 7.4 | 9.4 | reading |
| nettle-ink | #4E6716 | 5.9 | 5.3 | 2.2 | 2.8 | reading |

Each candidate brightens on soil-night, where weed spends most of its life: the table `weed
check` writes on a terminal runs on `#1C1610`, and the accent has to read as a word there.
bramble and woad clear 4.5:1 on soil-night as themselves. nettle only clears it in the night
value, and on paper it needs **nettle-ink `#4E6716`** for words — one more colour to carry, and
the reason it is third.

## What the accent is for, whichever one is settled

- The mark's shears — both blades and both handle loops. The plant stays growth green
  `#357E2C`, so the accent is only ever the thing doing the judging.
- The bloom dot in the garden row, and weed's row on the umbrella page.
- Block-level counts in the summary line, and rule ids in prose.

## What stays umbrella

Status colours are not weed's to redefine, and the temptation to make them weed's is the
whole trap. **Error `#BC4126` is never a finding.** A finding is a judgement about a change;
an error is weed failing to run, which is exit 3 and nothing else. Blocks take **caution
`#B0741C`**, a clean run takes **healthy `#46913C`**, and the accent marks *weed*, not
severity. A reviewer who learns to read the accent as "bad" has learned the wrong thing.

No other colours are added.
