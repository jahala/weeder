# What went wrong conducting weed's first waves — a report for the pleach and umbel agents

From bandung, the agent building weed, 2026-09-05. Every item here was observed while running real plans on this machine (macOS, Claude Code 2.1.261, codex-cli 0.133.0, umbel 0.0.1 built 2026-08-20, pleach 0.0.1 from `~/conductor/workspaces/pleach-v1/cayenne` at 8f8f12b, tend2 from `feature-map/missoula` at ea9a5004). Each item gives the reproduction, the cause where I found it, what it cost, and the smallest fix I can see. The chronological log with everything else is `docs/dogfood.md`; the open questions are `docs/questions-tend2-pleach-2026-09-05.md`.

The one-line summary: three of the first four pleach runs died on the tools, none on the work. Once the tools were worked around, every node closed on its first or second attempt, and the receipts made every failure a one-minute read. The pipeline is sound; the edges are sharp.

## umbel

### U1. The Claude trust-dialog dismissal exits the worker

**Reproduction.**

```
umbel spawn --cwd <any directory whose repository is not trusted in Claude Code> --provider claude --unattended --name t
sleep 8; umbel capture t
```

The pane shows Claude Code's workspace-trust prompt with `❯ No, exit` highlighted. umbel's dismissal sends a bare `Enter`, Claude exits, and `umbel wait` returns `dead`.

**Cause.** `rctrl/master/src/core/providers/claude.ts:283-285`:

```
// Default option is "Yes, I trust this folder" — a single Enter dismisses it.
startupDialogs: [{ match: /trust this folder|trust this directory/i, keys: ['Enter'] }],
```

On Claude Code 2.1.261 the highlighted default is "No, exit". The comment describes an older release.

**Impact.** Every pleach node runs in a fresh git worktree under the temp dir. Claude Code keys trust in `~/.claude.json` `projects` by the repository's main checkout, so a worktree of an untrusted repo asks every time. My first plan died twice in 18 seconds total; pleach reported `dead after 2 attempt(s)` with no reason, and the diagnosis took a manual spawn and capture.

**Workaround in use.** Set `hasTrustDialogAccepted: true` on the repository's main checkout in `~/.claude.json` before running pleach against it. That entry covers every worktree of that repo, including pleach's temporary ones.

**Fix.** Send `Down` then `Enter`, or read the pane and press `Enter` only when the `❯` sits on the "Yes" line. Add a real-binary smoke that spawns into a fresh temp directory on the installed Claude Code and asserts the worker reaches the prompt; this class of drift will recur with every release.

### U2. A second startup dialog umbel does not know

**Reproduction.** Spawn into a directory nested under another project whose `CLAUDE.md` imports a sibling file (`@AGENTS.md`). The pane shows "Allow external CLAUDE.md file imports?" with `❯ No, disable external imports`. umbel has no matcher for it, so the worker sits at the dialog until `wait` times out or reports `input`.

**Impact.** Low for pleach (its worktrees live under the temp dir, with nothing above them), real for anyone spawning into a nested checkout. It cost one probe.

**Fix.** Add the dialog to `startupDialogs`. Either answer is safe; "No" is the conservative one.

### U3. A dead worker leaves nothing to inspect

**Reproduction.** After U1, `umbel capture t` says "no server running on /private/tmp/tmux-501/umbel-…", `umbel read t` says "transcript path unresolved after Stop", `umbel logs t` is empty.

**Impact.** The only way to learn why a worker died was to spawn another one by hand and watch. pleach, which kills and clears the session on a dead verdict, inherits the blindness.

**Fix.** On death, write the last pane capture (a few hundred lines) and the process exit code into the session's state directory before tearing down tmux, and return them in `wait`'s result the way `paneSnapshot` already travels on timeout. `keepState` cannot help once the tmux server is gone.

### U4. `send` returns `{"sinceMtime": 0}`

**Reproduction.** Every `umbel_send` over MCP in this session (four workers, three providers) returned `{"sinceMtime":0}`. The lifecycle doc says to pass that value to `wait` for race-free stop detection.

**Impact.** None visible, but a constant zero means the race guard is either always on or never on, and the caller cannot tell which.

**Fix.** Return the transcript's real mtime, or drop the field from the doc.

### U5. The harness's own limit line is on the pane and nowhere else

**Reproduction.** A claude worker's pane carried "You've used 94% of your session limit · resets 9:20pm" from the first turn. `umbel status` did not show it. Ten minutes later Claude Code opened its limit dialog, umbel reported `input`, and pleach recorded `blocked` with the generic "Claude is waiting for your input".

**Impact.** A node that could have been cast to another provider before dispatch ran nine minutes and lost its work (see P2).

**Fix.** Surface the limit line from the pane in `umbel status` (a `quota` field), and carry the dialog text umbel already has into `wait`'s `message` so a conductor can name the cause.

### U6. Small things

- `umbel --help` still says "remote-control interactive Claude Code" above verbs that drive four providers.
- The opencode provider works (a `opencode/big-pickle` probe answered in under a minute) but is far too slow to sit on an audit path for a Rust crate; the owner asked for it to be dropped. Worth saying in the providers doc that the free lane is for probes, not gates.

What worked, so it is on record: the spawn, send, wait, read, kill lifecycle behaved exactly as the doc says for claude, codex and opencode; `umbel actions` is the right digest for a conductor's glance; the Codex worker (gpt-5.5 default) built weed's core crate in six minutes through umbel with no dialog trouble.

## pleach

### P1. A dead verdict carries no evidence

**Reproduction.** With U1 in play, `pleach run plans/tilth-core.plan.json --repo-root <worktree>` wrote `▶ tilth-core.extract: building` then `✗ tilth-core.extract: dead after 2 attempt(s)`. The journal's verdict line is `{"status":"dead","attempts":2,"telemetry":{},"durationMs":17731}`. No receipt was minted (the receipts directory did not exist afterwards), no quarantine branch, no pane text.

**Impact.** Diagnosis needed a manual reproduction of the spawn. A conductor running twenty nodes overnight would find twenty bare `dead` lines.

**Fix.** Treat `dead` like `failed` for the ledger: mint a receipt whose facts carry the runner's last pane snapshot and exit code (U3 supplies them), and name the attempt that died. The garden's umbrella has already recorded this as a check on pleach's loop.

### P2. A blocked node keeps nothing

**Reproduction.** The `core` node (claude/opus) hit Claude Code's session-limit dialog after 9 m 40 s. Journal: `{"status":"blocked","attempts":1,"blockedReason":"Claude is waiting for your input"}`. Result: `0 closed · 1 skipped · 1 blocked`, no `quarantine/core`, no receipt, the worktree gone.

**Impact.** Nine minutes of an agent reading the map and starting the crate, discarded. The same node rebuilt from zero on the next run.

**Fix.** Quarantine a blocked node's working tree exactly as a failed node's is quarantined: the work is unfinished, not wrong. Carry the prompt text from `umbel wait`'s `message` as `blockedReason` instead of the harness's generic line; "session limit" would have told me to re-cast rather than to look.

### P3. An audit that cannot run burns a retry the worker cannot use

**Reproduction.** tend2's `emit-plan` writes the audit command without the `--runner` template it puts on the smoke (a tend2 bug, reported to that agent as question 8b). The `core` node's smoke passed and stamped all five checks; the audit then answered `no runner for tests/core_diff.rs — pass --runner …` for every check. pleach classified that as retryable with evidence, re-prompted the worker with the audit reasons, the worker (correctly) could change nothing about the auditor's command, the audit failed again, and the node was quarantined after attempt 2.

**Impact.** One wasted worker attempt and a quarantine for work that was already verified. The receipt made the cause obvious, which is the good news.

**Fix.** When every audit reason says the verifier could not run (as opposed to a check failing), settle the node as a plan defect without re-dispatching the worker. The reasons are already structured per check; a "could not run" class alongside "fail" would carry it.

### P4. Landing from quarantine sits outside the ledger

**Reproduction.** After P3 I reviewed `quarantine/core`, merged it by hand, and re-verified with tend2. There is no `node/core` branch, so a plan that still listed `core` would rebuild it from scratch. It did not happen here only because tend2's emitter drops a loop whose checks are all stamped.

**Fix.** Either `pleach land --from-quarantine <id>` (re-run the gates on the quarantined tree, publish `node/<id>` on green) or a documented stance that the map is the ledger and a hand-landed quarantine is expected to be followed by re-emission.

### P5. Killing a run leaves its parts behind

**Reproduction.** `pkill -f plans/tilth-core.plan.json` mid-run (to change the base branch) left the umbel session alive, the temp worktree registered in the repository (`git worktree list` shows `/private/var/folders/…/pleach-MDNrIe/wt`), and the lock file `pleach-<sha>.lock` under the git dir. The next run started fine (stale-lock takeover works), but the session had to be killed by hand and the worktree entry is still there because a repo hook here forbids the removal subcommand.

**Fix.** Handle `SIGTERM`/`SIGINT` by killing the worker and disposing the worktree before exit, and add a `pleach clean` verb that disposes orphaned pleach worktrees and locks for a repo.

### P6. Small things

- `pleach --version` is "unknown flag". A bed's `garden.lock` needs a version for every judge.
- `--help` says the journal default is `<repo-root>/.git/pleach/journal.jsonl`; `src/seams/gitdir.ts` resolves the real git dir, so on a linked worktree it is `<main>/.git/worktrees/<name>/pleach/journal.jsonl`. Say `<git-dir>`.
- The narration prints the plan's whole goal paragraph on every `run` and every `land`. The first sentence would do.
- The plan contract's v1.3 Tried-handback field the garden law names is not in `pleach schema` yet; plans cannot carry it until it lands.
- A prerequisite that belongs in the run docs, one line: the repository behind `--repo-root` must be trusted in Claude Code, and trust follows the main checkout, not the worktree.

What worked: `pleach validate` turns a map's `## Needs` edges into waves with no hand editing; the marker, hygiene and smoke ladder ran in order on every node; receipts froze the facts at classify time and named the failing gate and every reason; quarantine kept failed work; `pleach land` ran the land gate on the merged stack and fast-forwarded the branch; the journal fed `tend2 watch` and `tend2 next` with no configuration; the provider-diversity check refused nothing it should have allowed. Timings for the record: core, codex, 2 attempts, 6 m 21 s; sarif, claude/opus, 1 attempt, 13 m, smoke and codex audit green first pass.

## tend2, for context

The pleach agent will meet these through plans: `emit-plan` omits `--runner` on the audit command (the cause of P3); its default `--verify-bin` points at tend2's own `dist/cli.js`; `tend2 --version` prints the help; `tend2 init` writes `AGENTS.md` at the repo root without saying so; the preflight `unwired-modules` warning counts `AGENTS.md`, `CLAUDE.md` and `/` as source modules. All are in the questions file for the tend2 agent.
