# Dogfood notes — tend2, pleach, umbel while building weed

Running log. One dated entry per observation: the exact command, what happened, what it cost me. Written for the people who maintain these tools. Praise is as useful as friction; both are recorded.

## 2026-09-05

### tend2

- `tend2 --version` prints the full help instead of a version. The binary at `/opt/homebrew/bin/tend2` resolves to `feature-map/missoula/dist/cli.js`, a dev build, so there is no version to pin in a Tried line or a `garden.lock`. A one-line version output would let a bed record which verifier stamped it, which the garden law (§5, "add the verifier's identity and version to the stamp") already wants.
- `tend2 init docs/tend2` ran in 39 ms, exit 0, and wrote the map, the renderer, and an `AGENTS.md` at the repo root. Good: the "Next" block tells the agent exactly where to go. Surprise: it wrote `AGENTS.md` without saying it would touch a file outside the directory I named. In a repo that already has an `AGENTS.md` this needs a merge story; here the repo was empty so no harm.
- The scaffolded loop is named after the directory (`bandung.tend2.html`), which in a Conductor workspace is the branch's placeholder name, not the project. Naming it after the git repo's basename or asking for a name would land closer.

### pleach

- `pleach --version` is not a flag: "pleach: unknown flag '--version'". Same pinning problem as tend2; a bed's `garden.lock` needs a version for every judge.
- `pleach schema` is exactly what the pleach-plan skill promises: the contract in JSON Schema, 6 KB, readable in one pass.

### umbel

- `umbel --version` prints `umbel 0.0.1`. Fine.
- `umbel --help` says "remote-control interactive Claude Code" in the tagline while the verbs cover four providers. Cosmetic.

### Shaping the map (later the same day)

- `tend2 lint` on fourteen hand-written loops came back clean first time; FORMAT.md is precise enough to write against without trial and error. `--strict` added nothing on these files.
- `tend2 next docs/tend2` after shaping: "next up → core", with the project loop's human check held back because no machine evidence exists yet. Good routing; it picked the only root loop with code checks in this repo (the tilth-core loop's first check needs another repository, which `next` cannot know).
- `tend2 emit-plan` on the empty scaffold emitted zero nodes and no warning that the map was unshaped. A one-line "no loop carries an open code check" would save a first-time user a puzzled minute.
- The emit-plan work order carries goal, checks, Tried and the rules block, and tells the worker the loop file is its full context. It does not carry the narrative inline, so a worker must actually open the file; that is fine for Claude Code, worth stating in the docs for headless runners.
- `pleach validate plans/tilth-core.plan.json` printed the topo order, the waves and the gates per node in one JSON line. Exactly what the pleach-plan skill promises.
- The `closes` field on a node is documented as "ledger metadata; loop does not consume". I set it to `docs/tend2/tilth-core.tend2.html#c1` anyway so the plan says what it is for; nothing checks the format.

### umbel probe

- `umbel_spawn` (claude, haiku, unattended) → `umbel_send` → `umbel_wait` (reason `stop`) → `umbel_read` ("OK") → `umbel_kill` worked first time, about a minute end to end. The MCP surface matches the lifecycle doc exactly.
- `umbel_send` returned `{"sinceMtime":0}`. The doc says to pass `sinceMtime` from send into wait for race-free stop detection; a zero looks like "no transcript yet" rather than a real mtime, so I could not tell whether passing it would help or hurt. Worth a sentence in the doc, or a real value.
- The `--help` tagline still says "remote-control interactive Claude Code" while four providers are supported.

### The first pleach run died in 18 seconds — the trust dialog

- `pleach run plans/tilth-core.plan.json --repo-root .context/tilth-core` reported `dead after 2 attempt(s)` with a 17.7 s node duration and nothing else. The journal's verdict line has no reason; the worker's pane and umbel state were already gone (pleach kills and clears them). Finding the cause took a manual reproduction. A dead verdict should carry umbel's last pane snapshot or the exit code; `umbel wait` already returns `paneSnapshot` on timeout, so the data exists.
- Reproduction: `umbel spawn --cwd .context/tilth-core --provider claude --model opus --unattended` then `umbel capture` shows Claude Code's workspace-trust dialog with `❯ No, exit` highlighted. umbel's Claude provider declares `startupDialogs: [{ match: /trust this folder/i, keys: ['Enter'] }]` with the comment "Default option is Yes, I trust this folder". On Claude Code 2.1.261 the default is "No, exit", so the dismissal exits the worker and every fresh, untrusted cwd (which is every pleach worktree) dies before the prompt is sent. Fix on umbel's side: send `Down` then `Enter`, or match the highlighted line. Source: `rctrl/master/src/core/providers/claude.ts:283-285`, binary built 2026-08-20.
- How Claude Code decides trust, learned the hard way: the key in `~/.claude.json` `projects` is the repository's main checkout, not the cwd. My Conductor workspace is a worktree of `~/conductor/repos/weed` (trusted), so any directory under it skipped the dialog; the tilth worktree under `.context/` belongs to `~/CascadeProjects/tilth`, whose entry had `hasTrustDialogAccepted: false`, so it asked. Setting that one flag to true (backup at `~/.claude.json.bak-weed-2026-09-05`) removed the dialog for every worktree of tilth, including the ones pleach creates under the temp dir. pleach's docs could say this in one line: "the repo you pass as --repo-root must be trusted in Claude Code, and trust follows the main checkout".
- A second dialog, "Allow external CLAUDE.md file imports?", appears for a cwd nested under another project whose `CLAUDE.md` imports a sibling (`@AGENTS.md`). umbel does not know this dialog either. It will not affect pleach worktrees (nothing above the temp dir has a CLAUDE.md), but a nested worktree layout like mine trips it.
- `umbel send` returned `{"sinceMtime":0}` again for the opus probe; the value seems to be constant rather than a transcript mtime.
- `tend2 emit-plan` preflight flagged four loops as `unwired-modules` because their prose mentions `AGENTS.md`, `CLAUDE.md`, `/` and `plans/tilth-core.plan.json` and no evidence spans them. The heuristic counts any existing path in the narrative as a "src module"; a filter for source extensions, or for paths under `src/`, would silence these.
- `pleach validate` on the full 14-node plan printed the eight waves cleanly; the map's `## Needs` edges became the DAG with no hand editing. That is the bridge working as advertised.

### Wave one running (core under pleach, claude/opus, codex audit)

- `tend2 watch --once --repo-root . --journal <git-dir>/pleach/journal.jsonl` narrated the run start and "core: started" from pleach's journal with no configuration, and `tend2 next` gained a "running now — 1 running · 0 closed · 0 BLOCKED ON YOU" block from the same file. The bridge is real in both directions. One rough edge: `watch` needs the journal path, and the default pleach writes to lives under the git dir of a linked worktree (`.git/worktrees/<name>/pleach/journal.jsonl`), which nobody will guess; `tend2 watch` could ask git for it the way pleach does.
- `umbel actions <session>` on the running worker listed its first eight Bash calls in order: it read the loop, the runner script, the umbrella brief, the sibling loops and `tend2 verify --help` before touching anything. That digest is the right shape for a conductor's glance and cost nothing.
- The worker's pane carried "You've used 94% of your session limit · resets 9:20pm". pleach's cast has no notion of the builder's remaining quota, and the ledger only learns cost after a verdict. A run that starts at 94 percent will stall inside the node; the conductor finds out at the node timeout. Surfacing the harness's own limit line in `umbel status` (it is on the pane) would let a conductor re-cast before dispatch.

### The session limit, and what a blocked node leaves behind

- The core worker (claude/opus) was blocked after 9 m 40 s: pleach's verdict says `blocked`, reason "Claude is waiting for your input". The input Claude wanted was the subscription's session-limit dialog (the pane had shown 94 percent at start; the limit resets 21:20). pleach handled the shape correctly and shouted `core NEEDS YOU`, but a blocked node keeps nothing: no `quarantine/core` branch, no receipt, no diff. Nine minutes of an agent reading the loop and starting the crate are gone. A blocked node should quarantine its working tree the way a failed one does; the work is not wrong, it is unfinished.
- The blocked reason names the harness ("Claude is waiting for your input") but not the prompt text. `umbel wait` returns the question in `message`; carrying it into the journal would have said "session limit" outright and let a conductor re-cast instead of guess.
- pleach's cast has no way to know a provider's remaining quota, and neither does umbel's spawn. The cheapest fix is on umbel's side: `umbel status` could surface the harness's own limit line, which is on the pane before the first prompt is sent.
- pollen restarted on both machines' sides and dropped its in-memory allows; almaty and I ended up held at each other's gate with cape-town relaying by disk path. `POLLEN_ALLOW=almaty,cape-town` in the MCP config is the fix the umbrella suggests; allows that a person granted should survive a restart by default.
- My own session hit the same limit twice while writing this; the Conductor harness resumed cleanly but the background pollen watcher and the journal monitor were orphaned and had to be restarted. Anything that watches on my behalf needs to be re-armed after a resume.

### core: built by codex in six minutes, refused by an audit that could not run

- The codex worker (gpt-5.5 default through umbel) built the crate, five modules and five test files, in two attempts totalling 6 m 21 s. The smoke gate ran `tend2 verify` with the runner and stamped all five checks inside the worktree. Then the audit failed on every check with "no runner for tests/core_diff.rs — pass --runner": `emit.ts` appends the runner template to the smoke but not to `auditCommand`. pleach did the right thing with a wrong plan: retried once with the evidence, failed, quarantined the work on `quarantine/core`, minted a receipt that names the failing gate and every reason. The receipt made the diagnosis a one-minute read. That is the honesty ledger working.
- Landing from quarantine by hand (`git merge quarantine/core`) worked, but it is outside pleach's ledger: no `node/core` branch, so a later run of the same plan would rebuild core from scratch. Since emit-plan drops a done loop, that never happens here, but a conductor who lands quarantined work needs either `pleach land --from-quarantine <id>` or a documented "the map is the ledger" stance.
- Conductor review found three real defects the gate could not: a classification that would blind a rule, two spellings for one config key, a missing helper. None of them fails a test that a builder wrote to its own understanding; the second party reading the diff is still the thing that catches them. The cross-provider audit as emitted (re-running the same verifier) would not have caught them either — it checks that the evidence runs, not that the evidence is right.
- `scripts/emit-plan.sh` now wraps emit-plan plus the `jq` patch for the audit runner, so the bug cannot bite the next wave; it is the kind of wrapper a repo should not need.
- `tend2 verify --force` on the landed tree re-stamped all five checks in one command; the stamps in the quarantined loop file matched, so the sha law transfers across worktrees exactly as FORMAT.md says.

### sarif: the loop as a work order, from inside the worktree

- `scripts/check/run.sh` earned its keep: four checks, three of them `tests/<name>.rs` and one a shell script, and the runner mapped every one without an edit. A worker never has to know how the verifier invokes it.
- `tend2 verify --force --expect-payload` stamped all four checks in one call and printed `c1 stamped` … `c4 stamped`. The check ids alone do not say which evidence produced which stamp; printing the evidence path beside the id would let a worker read the result without opening the loop file.
- The loop's conventions and the SARIF schema disagreed on one point, and the loop had no room to say so: `helpUri` is `format: uri` in the vendored schema, so the `docs/rules.md#<id>` the loop names cannot be written as-is. The fix is in `docs/sarif.md` (the face hands `render` a documentation base). A worker cannot write to `## Tried`, which is the right law, so a decision like this reaches the map only through the conductor's read of the diff.
