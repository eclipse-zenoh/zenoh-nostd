# Contributing to zenoh-nostd

Thanks for your interest in zenoh-nostd! This guide covers what you need to
get a pull request (PR) merged. By contributing, you agree that your work is
licensed under the project's licenses (EPL-2.0 OR Apache-2.0), and to follow
our [Code of Conduct](CODE_OF_CONDUCT.md).

## Before your first PR: sign the ECA

zenoh-nostd is an [Eclipse Foundation](https://www.eclipse.org/) project. We
can only accept contributions from people who have signed the
**Eclipse Contributor Agreement (ECA)**. You do this once, and it covers all
Eclipse projects.

1. [Create an Eclipse Foundation account](https://accounts.eclipse.org/user/register).
2. [Sign the ECA](https://accounts.eclipse.org/user/eca) while logged in
   ([what the ECA says](https://www.eclipse.org/legal/ECA.php)).
3. Commit with the **same email address** as your Eclipse account:
   `git config user.email "you@example.com"`.
4. Sign off your commits with `git commit -s`. This adds a
   `Signed-off-by: Your Name <you@example.com>` line to the commit message.

An automated ECA check runs on every PR. If it fails, the commit author email
usually doesn't match your Eclipse account. Fix the email, then amend or rebase
your commits (`git rebase --exec 'git commit --amend --no-edit --reset-author -s' main`)
and force-push.

## Golden rules

1. **Talk before big changes.** For a new feature, an API change or a large
   refactor, open an issue first so we can agree on the approach before you
   write the code.
2. **One PR, one change.** Small, focused PRs get reviewed and merged faster.
   Don't mix a refactor, a fix and formatting changes in one PR.
3. **Start from an up-to-date `main`.** Rebase on `main` before opening the PR
   and when there are conflicts.
4. **Make CI green before asking for review.** Run the checks below locally.
5. **Respect the project's constraints.** The core crates are `#![no_std]` and
   don't allocate. Anything that needs the heap goes behind the `alloc`
   feature. PRs that make the benchmarks slower should be avoided (`just bench`).
6. **Add tests and docs.** If you fix a bug, add a test that would have caught it.
   If you change public API, update its documentation and the examples.
7. **Explain why, not just what.** In the PR description, say what problem the
   PR solves and how you tested it. Link the issue (`Closes #123`).
8. **Give your PR a Conventional-Commit title.** It sets the changelog label
   automatically (see below).
9. **Be patient and kind.** Reviewers are often volunteers. Reply to comments,
   push fixes as new commits during review, and say when the PR is ready again.

## Checks to run before opening a PR

These match what CI runs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`just check` runs clippy on every feature and platform, including wasm and
ESP32 (it needs those toolchains installed).

## PR titles & labels

Release notes are generated from PR labels (see `.github/release.yml`), and every
PR needs at least one changelog label to pass the `Check PR labels` check.

You don't need to set labels yourself. Give your PR a
[Conventional Commits](https://www.conventionalcommits.org/) title,
`type(scope)!: subject` (scope and `!` are optional), and the label is added
automatically:

| Title | Label |
|---|---|
| `feat: …` | `enhancement` |
| `perf: …` | `enhancement` |
| `fix: …` | `bug` |
| `docs: …` | `documentation` |
| `deps: …`, or any type with scope `deps` (`chore(deps): …`, `build(deps): …`) | `dependencies` |
| `ci:`, `chore:`, `refactor:`, `test:`, `style:`, `build:`, `revert:` | `internal` |
| `!` before the colon (`feat!: …`), or a `BREAKING CHANGE:` line in the PR description | also `breaking-change` |

Examples:

- `feat(session): add a builder to configure sessions` → `enhancement`
- `fix(transport): handle EOF on TCP read` → `bug`
- `refactor(proto)!: rename ZBytes to Bytes` → `internal` + `breaking-change`
- `Fixed some stuff` → no label; the check fails

Rules for the title:

- Write the type in lowercase (uppercase works too), followed by `: ` (a colon
  **and a space**).
- Keep the subject short, in the imperative ("add", not "added"), with no
  final period.
- The scope is optional. Use it to name the area you changed (`session`,
  `proto`, `embassy`, …).

If you rename the PR, the label follows the new title. If the title doesn't
follow this format, or no type fits, ask a maintainer to add a label. The
automation never changes labels set by a maintainer.
