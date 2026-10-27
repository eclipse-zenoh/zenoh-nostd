# Contributing to zenoh-nostd

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

The type is case-insensitive. If you rename the PR, the label follows the new title.

If the title doesn't follow this format, or no type fits, ask a maintainer to add
a label. Labels set by a maintainer are never changed by the automation.
