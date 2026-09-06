# weeder — voice delta

Inherits the plotplot umbrella voice (calm · precise · literate · a little wit). One signature
line and a terminology table, merged with the umbrella at read time.

**Signature phrase:** *pulled before it takes root.*

## Terminology

| Use | Not | Why |
|---|---|---|
| finding | violation, error, issue | weeder reports what it found; the severity says how much it matters. |
| block / warn / note | fail, pass, critical, info | The three levels the rules use; say them the same way in prose. `block` is the only one that stops anything. |
| rule | check, policy, lint | A rule has an id and a fixture. "Check" is tend2's word for a claim; do not borrow it. |
| judge | reviewer, linter, scanner | weeder judges a diff; it does not review code or scan for style. |
| allowance | suppression, ignore, override | A person let a finding through, on the record. `Weed-allow` is the trailer; the allowance is a finding too. |
| the diff / the tree | the change, the PR, the code | `check` reads the diff; `scan` reads the tree. Name which. |
| guard | hook, protection, policy | The git-hook face: the law at commit and push. |
| bite | mutation test, negative control | The revert-and-run proof that a test still fails without the change. |
| honest work | clean code, quality | What weeder can decide: that the tests still mean what they meant. It says nothing about whether the code is good. |

Findings say what was found, why it matters, and the next action:
`T1 test deleted: tests/auth.spec.ts lost 3 cases. Green no longer proves what it proved.
Restore them, or allow with a reason: Weed-allow: T1 <reason>.`

Product name is lowercase always: `weeder`. The metaphor stays in the mark and the tagline;
findings never joke, and nothing is ever "weedered out" in interface copy.
