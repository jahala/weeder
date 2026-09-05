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
