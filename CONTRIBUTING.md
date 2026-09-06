# Contributing

Thanks for your interest. weeder is small on purpose: one core, three faces, and a rule catalogue where every rule is proven by a fixture. Clean, focused changes are the easiest to land.

## Workflow

1. Fork, branch, change.
2. Run the gates locally:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
3. Open a PR. Say what changed and how to test it.

CI runs the same three commands on every change, and runs the suite again on a release build so the latency budget is measured where it counts.

## What helps

- **A fixture first.** A new rule, or a change to what one fires on, lands as a fire and a silent fixture under `fixtures/adversarial/<RULE>/<lang>/` in every language weeder reads, then the detector. A test that cannot fail is not a test.
- **Nothing mocked.** Tests build a real git repository in a temp dir and run the built binary. `tests/common/mod.rs` is the harness; use it rather than a shortcut.
- **Small PRs.** Easier to review, easier to merge.
- **A commit body that says why.** The log is the project's reasoning trail; the subject says what, the body says why.
- **Surgical edits.** Leave surrounding code alone unless the change needs it.

## The map

Work here is planned on loops. `docs/tend2/` holds one page per module with the checks that prove it, and `AGENTS.md` carries the layout, the dependency direction weeder enforces on itself, the voice, and the rules any change is held to. For anything that adds a rule, a face or a flag, read the loop that owns it first; for anything the map has no loop for, open an issue so the shape is agreed before the code.

## Code style

Match the file you are editing. Core does no I/O and never panics on input; failure lives in the return type. User-facing strings are sentence case, product names lowercase, no exclamation marks, and a finding says what was found, why it matters, and the next action, in that order.

## License

By contributing, you agree your work is licensed under the project's [MIT License](./LICENSE).
