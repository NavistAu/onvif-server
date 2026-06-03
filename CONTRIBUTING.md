# Contributing to onvif-server

## Prerequisites

`onvif-server` depends on the `soap-server` crate via a path dependency. For local
builds and tests you need both repositories checked out side-by-side:

```
~/ws/
  onvif-server/   ← this repo
  soap-server/    ← sibling checkout required for local builds
```

Clone soap-server:

```
git clone https://github.com/NavistAu/soap-server ~/ws/soap-server
```

## Build, test, lint

```bash
# build
cargo build

# run the test suite
cargo test

# run tests with the discovery feature enabled
cargo test --features discovery

# lint (must be clean before opening a PR)
cargo clippy --all-targets --all-features -- -D warnings

# format check
cargo fmt -- --check

# apply formatting
cargo fmt
```

## Branching model (gitflow)

| Branch              | Purpose                                               |
|---------------------|-------------------------------------------------------|
| `main`              | Published releases only. Tagged automatically by CI.  |
| `develop`           | Integration branch. All feature work targets here.    |
| `feature/<name>`    | New features and non-trivial changes.                 |
| `fix/<name>`        | Bug fixes.                                            |
| `release/vX.Y.Z`   | Release preparation off `develop`; PR into `main`.    |

Workflow:

1. Branch from `develop`: `git checkout -b feature/my-thing develop`
2. Commit using Conventional Commits (see below).
3. Open a PR targeting `develop`.
4. CI must be green before merge.
5. Releases are prepared on a `release/vX.Y.Z` branch, then PR'd into `main`.
   Merging to `main` auto-tags and publishes to crates.io via Trusted Publishing.

## Conventional Commits

Commit messages must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <short description>

[optional body]

[optional footer]
```

Common types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`.

Examples:

```
feat(ptz): add goto_preset support
fix(discovery): handle malformed probe messages gracefully
docs: update quickstart example
```

Breaking changes: append `!` after the type/scope, e.g. `feat!: rename builder method`.

## Pull request requirements

- Target branch is `develop` (not `main`).
- All CI checks must pass (build, test, clippy, fmt).
- New public APIs must have rustdoc comments.
- New behaviour must be covered by tests where practical.
- Keep commits focused; squash noise before opening the PR.

## Reporting issues

Open a GitHub issue at <https://github.com/NavistAu/onvif-server/issues>.

For security vulnerabilities, **do not open a public issue** — see
[SECURITY.md](SECURITY.md).
