# Security Policy

## Reporting a vulnerability

Please **don't** open a public issue. Use GitHub's private advisory flow:

<https://github.com/jahala/weeder/security/advisories/new>

We'll acknowledge within 72 hours and coordinate disclosure with you.

## Supported versions

Only the latest minor release receives security updates. Older versions don't.

## What weeder does with your code

weeder reads a diff and the files it touches, runs `git` with argument arrays (never a shell string), and writes a SARIF log. It makes no network requests and sends nothing anywhere, with one exception: `weeder scan --refresh-snapshot` asks curl to reach the package registries, and it says so. `weeder scan` also runs each program that `[docs] commands` in `weeder.toml` names, with `--help`, to check the commands the docs cite.

A secret weeder finds is never repeated in its output: the finding names the shape and the line, not the value.

## Security testing

weeder runs:

- **Unit and integration tests** on every change (`cargo test`), including a property test over the diff parser and the classifier with arbitrary and mutated inputs, and a suite that drives the real binary on repositories built to hurt (symlinks, submodules, binary and non-UTF-8 files, 20 MiB files, paths with quotes).
- **A panic hook** that turns any panic into exit 3 with a one-line reason, so a bug in weeder fails closed rather than letting a change through.
- **Determinism checks**: the same diff, tree and config produce byte-identical SARIF across runs and environments.
