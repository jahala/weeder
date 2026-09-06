# Dogfood notes — the hooks loop

One worker, one loop (`docs/tend2/hooks.tend2.html`), run through pleach into an
isolated worktree. Written for the people who maintain tend2 and pleach.

## 2026-09-05

### tend2

- `tend2 verify docs/tend2/hooks.tend2.html --repo-root . --force --expect-payload …
  --runner 'bash scripts/check/run.sh {evidence}'` did exactly what the work order
  said it would, in one pass: four code checks stamped, the human check reported
  as `skipped-human`, exit 0. Being handed the gate command verbatim in the work
  order is the single most useful thing about working a loop, there is no
  guessing about what "done" means.
- The runner in `scripts/check/run.sh` already mapped `*.sh` to `bash`, so a check
  whose evidence is a shell script that starts a real agent session needed no
  change to the plumbing. Worth keeping: the evidence contract being "a path"
  rather than "a cargo test" is what let a proof-by-real-session be first-class
  evidence next to the unit tests.
- A stamp keys on the evidence file's content. `scripts/proof/claude-stop.sh`
  writes its capture into `docs/proof-2026-09.md`, so the stamp says the script
  ran and stays silent about whether its output is still there. Deleting the
  capture would leave a green check pointing at nothing. A check whose evidence
  writes a file elsewhere might want to declare that file too.

### The loop as a brief

- The `## Tried` line saying the pending commit message reaches `weed check`
  through `--message-file`, "which is the only way a `Weed-allow:` trailer counts
  before the commit exists", does not survive contact with the code. The hooks
  judge with `--strict`, and `--strict` gives a suppressed finding its own level
  back, so a trailer handed in changes nothing a hook decides. weed therefore
  does not lift `-m` text out of the command line, and the reasoning is recorded
  in `docs/proof-2026-09.md` instead. Plumbing whose effect no test can observe is
  plumbing that rots.
- The `## Proof` section asks for "a fixture repo whose tree deletes a test". No
  rule detects a deleted test yet; G1 (a committed conflict marker) is the only
  thing that makes a tree red today, and the loop's own `## Tried` says as much
  two paragraphs later. The proof uses a half-finished merge. A shaped loop that
  contradicts itself between sections costs a worker one read to resolve.
- The brief named Claude Code and Gemini CLI event shapes but left codex as "read
  its current hook documentation in the installed CLI". Codex ships no hooks
  documentation; it ships JSON schemas for every hook event's input and output
  inside its binary, which is better than documentation and took a `strings` pass
  to find. That is now written down in `docs/proof-2026-09.md` so the next worker
  does not repeat the dig.

### pleach

- The worktree arrived clean, at the repository root, on a detached HEAD, with the
  loop file read-only in practice. Nothing to report: it stayed out of the way for
  the whole run, which is the compliment.
- `BLOCKED.md` is no longer in `.gitignore`, so the P7 defect from the first
  attempt at this loop cannot repeat here.

### umbel

Not used this run.
