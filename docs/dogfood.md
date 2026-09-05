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
