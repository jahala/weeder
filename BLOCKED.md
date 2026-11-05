# Blocked: this repository's own weeder.toml cannot move to `[scope] exclude` through the installed guard

Check 13 of `docs/tend2/dogfood-2.tend2.html` (#99, #85) is implemented and committed, except one
file: `weeder.toml` at the root still says `specimens = ["fixtures/adversarial"]`.

## Why

The guard installed in this repository's shared hooks runs `/Users/jahala/.local/bin/weeder`, which
is 0.3.0. Its pre-commit hook parses the `weeder.toml` a change writes (`written_law` in
`src/faces/mod.rs`) and refuses the commit when it does not parse:

```
weeder.toml as this change writes it is not valid: weeder.toml: unknown field `exclude`, expected `allow` or `specimens`. fix that key, or drop it for the default.
```

The new binary refuses `specimens` the same way, because #99 keeps no alias. So no commit can carry
the move while the hooks run 0.3.0, and the worker may not bypass the hooks, change
`core.hooksPath`, or replace the shared binary other worktrees run.

## What breaks until it lands

- `tests/self_check.rs` fails: it judges the tree under the tree's own `weeder.toml`, which the new
  binary refuses with exit 3.
- `weeder check` built from this branch exits 3 on this repository: the law at the base names a key
  it no longer reads. `--config` naming a law with `exclude` is the way through for one run.

## What lands it

Install the guard from a build of this branch (`weeder guard install` with that binary on the path
the hooks name), then commit this change with the trailer below. Every other worktree whose base
still carries `specimens` then needs the same move before it can commit, because the new binary
refuses the old key.

```diff
 # The adversarial fixtures are written to look dishonest, so weeder reads none of
-# them. Every path this covers still comes back as a note, once for the path, so
-# the exclusion is as visible in the log as an allowance would be.
+# them. The pattern still comes back as a note counting the files it set aside,
+# so the exclusion is as visible in the log as an allowance would be.
 [scope]
-specimens = ["fixtures/adversarial"]
+exclude = ["fixtures/adversarial/"]
```

Commit message:

```
Move weeder's own law from specimens to [scope] exclude

The specimens key is gone with no alias (#99), so this repository's own
weeder.toml names its fixtures under [scope] exclude, the key every other
repository now writes. The change is a guardrail edit, allowed below, and a
person reviews it on the pull request.

Weeder-allow: C1 the repository's own law moves from the retired specimens key to [scope] exclude, which #99 specifies

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

Then delete this file in the same commit.
