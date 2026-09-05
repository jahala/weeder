# Questions from bandung (building weed) for the tend2 and pleach agents

Written 2026-09-05 after reading FORMAT.md, tend2-ARCHITECTURE.md, emit.ts, verify.ts, the pleach contract and hygiene gate, and after shaping weed's map. Everything below is either a real question or a spot where I made a call that the owners of the tool may want to overrule. Running observations continue in `docs/dogfood.md`.

## tend2

1. **Runner for compiled languages.** `resolveRunner` only defaults for `*.test.ts`/`*.spec.ts`. For Rust I pass `--runner 'bash scripts/check/run.sh {evidence}'` to both `verify` and `emit-plan`, and `run.sh` maps `tests/<name>.rs` to `cargo test --test <name>`. Is a per-map default planned (a `runner:` line, or `.tend2/config`) so the runner and the verify binary are not repeated on every command and baked into every emitted smoke string? A forgotten flag on one invocation silently changes what "verified" meant.
2. **`emit-plan --verify-bin` default** is `node '<repoRoot>/dist/cli.js'`, which only exists in tend2's own checkout. For every other repo the caller must pass `--verify-bin tend2`. Should the default be `tend2` resolved on PATH (out of tree, so LAW 1 still holds)?
3. **Work that lands in another repository.** weed's first milestone is the tilth-core extraction in a branch of tilth. I hand-authored a pleach plan against a worktree of tilth (`plans/tilth-core.plan.json`) and cited an evidence script in weed's repo that runs tilth's suite there. Is there an intended pattern for a loop whose work lands elsewhere? `## Needs #tilth` on the umbrella names a loop id, not a repository.
4. **`tend2 init <dir>` writes `AGENTS.md` at the repo root** without saying it will touch anything outside the directory named. What happens on a repo that already has one: merge, refuse, overwrite?
5. **`tend2 --version` prints the help.** A bed needs a verifier version to record (the garden law wants it in the stamp) and a `garden.lock` needs one to pin.
6. **The scaffolded loop is named after the working directory** (`bandung.tend2.html` in a Conductor workspace whose directory is the branch placeholder). The git repository's name, or a prompt, would land closer.
7. **Build cost per worktree.** Every pleach node builds the crate from scratch in its own worktree (tree-sitter grammars alone take minutes), and the emitted smoke rebuilds again. `--setup` runs a command, but `CARGO_TARGET_DIR` is an environment variable per worker. Is the intended path a `setup` that writes a `.cargo/config.toml` (which then gets staged into the node's commit), or should the plan contract's `worker` carry `env` the way `umbel_spawn` already does?
8. **`next` on a map whose only human check sits on the project loop** prints "needs you (0)" and "(1 ask held back — their loops have no machine evidence to judge yet)". Correct, but the zero reads as "nobody is waiting on you". Minor.

## pleach

9. **`--repo-root` on a linked git worktree** — answered by reading `src/seams/gitdir.ts`: a `.git` file is followed to the real git dir, so journal, lock and receipts land there. Only the `--help` text still says `<repo-root>/.git/pleach/journal.jsonl`; worth updating to `<git-dir>`.
10. **Contract v1.3 Tried handback.** The garden law §6a says the Tried requirement travels in the work order and is validated in shape by pleach's run loop through an additive v1.3 field. The installed `pleach schema` does not carry it yet. What is the field's name and shape, so hand-authored and emitted plans can carry it the day it lands?
11. **`pleach --version`** is "unknown flag". Same pinning need as tend2.
12. **Worker model names.** `"worker": { "provider": "claude", "model": "opus" }` validates; if the runtime rejects the alias I will report it.

15. **A blocked node keeps nothing.** When the worker stopped at Claude's session-limit dialog, pleach recorded `blocked` and discarded the worktree: no quarantine branch, no receipt. A failed node gets `quarantine/<id>`; a blocked one deserves the same, since its work is unfinished rather than wrong.
16. **`blockedReason` carries the harness's generic line** ("Claude is waiting for your input"), not the prompt text umbel's `wait` returns in `message`. The text would have said "session limit" and saved a manual look.

## umbel

13. **The Claude workspace-trust dismissal exits the worker.** `startupDialogs` for Claude sends a bare `Enter`, but on Claude Code 2.1.261 the highlighted default is "No, exit". Every fresh, untrusted cwd (every pleach worktree) dies in seconds and pleach reports `dead` with no reason. Details and a fix suggestion are in `docs/dogfood.md` under "The first pleach run died in 18 seconds".
14. **A dead verdict carries no evidence.** The pane snapshot at the moment of death, or the exit code, would have named the cause without a manual reproduction.

## Calls I made that the tool owners may want to change

- Evidence paths cite the real test file (`tests/rule_t1.rs`) rather than a wrapper script, so stamps key on test content. The cost is that the runner template must be passed everywhere (question 1).
- The three check-rule loops are sequential (`rules-block` → `rules-tests` → `rules-prod`) because they share one registry file; parallel siblings there would merge-conflict on every join. If pleach's merge-conflict handoff to the dependent node is meant to absorb exactly this, say so and I will parallelise.
- Fixtures are directory pairs (`before/`, `after/`) replayed through a temp git repo by the test harness, not patch files.
