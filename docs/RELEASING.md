# Releasing

Releases take two manual steps on `main`. The **Version** workflow opens a version PR that applies the pending changesets, and a maintainer reviews and merges it. A separate **Release** workflow then publishes the prepared version; it is added in a later PR of [#22](https://github.com/intermezzo-chess/scidtopgn/issues/22). Merging to `main` never versions or publishes anything on its own.

## How versions work

- `scidtopgn` and `scidtopgn-cli` are released together. They share the version in `[workspace.package]` in the root `Cargo.toml`, and each release gets one `vX.Y.Z` tag.
- Versions come only from changeset files in `.changeset/`. Commit messages don't affect them.
- [Knope](https://knope.tech) applies the changesets. It is configured in [`knope.toml`](../knope.toml) and pinned to 0.23.0 by `KNOPE_VERSION` in [`version.yml`](../.github/workflows/version.yml).

Preparing a version updates:

- `version` in `[workspace.package]` in `Cargo.toml`
- the `scidtopgn` and `scidtopgn-cli` entries in `Cargo.lock`
- the CLI's dependency on `scidtopgn` in `crates/scidtopgn-cli/Cargo.toml`
- `CHANGELOG.md`, which gets a section for the new version

It also deletes the changesets it used.

## Record a change

Contributors do this on their branch. See also [CONTRIBUTING.md](../CONTRIBUTING.md#record-the-change).

1. Install Knope 0.23.0, either with `cargo install knope --version 0.23.0 --locked` or from its [release page](https://github.com/knope-dev/knope/releases/tag/knope%2Fv0.23.0).
2. Run `knope document-change` from the repository root. Choose the change type, then enter a one-line summary.
3. Optionally, add details below the heading in the new `.changeset/*.md` file.
4. Commit the file with the change.

A changeset looks like this. `default` is the name Knope gives the single package:

```markdown
---
default: patch
---

# Fix a crash when the name base is empty

Optional details, in Markdown.
```

A summary without details becomes a bullet in the changelog. A changeset with details gets its own heading. Only `.md` files belong in `.changeset/`, and each must have this front matter.

Add a changeset for changes that users of the library or the CLI can notice. Changes to tests, CI, or contributor documentation don't need one.

### Choosing patch, minor, or major

| Type    | Use it for                                                                                                      | Changelog section |
| ------- | --------------------------------------------------------------------------------------------------------------- | ----------------- |
| `major` | Breaking changes, such as removing or renaming a public API item or CLI option, or changing output incompatibly | Breaking Changes  |
| `minor` | Backward-compatible additions, such as a new API item or CLI option                                             | Features          |
| `patch` | Bug fixes and other compatible changes that users can notice                                                    | Fixes             |

The highest type among all pending changesets decides the bump.

### How 0.x versions are bumped

While the major version is `0`, Knope treats the minor number as the breaking-change number. Checked against Knope 0.23.0, starting from `0.1.0`:

| Highest pending type | Next version |
| -------------------- | ------------ |
| `patch`              | `0.1.1`      |
| `minor`              | `0.1.1`      |
| `major`              | `0.2.0`      |

So `minor` and `patch` both bump the last number, and only `major` bumps the middle one. Still choose the type that describes the change: from `1.0.0` on, the three types bump the three numbers. Moving to `1.0.0` is a separate maintainer decision and isn't automated.

## One-time setup

The Version workflow pushes the version branch and opens the PR with the workflow's own `GITHUB_TOKEN`. No personal token or secret is needed.

In the repository, open **Settings → Actions → General**. Under **Workflow permissions**, turn on **Allow GitHub Actions to create and approve pull requests**. Without it, the workflow fails at **Create or update the version PR**. Keep the default permissions read-only; the version job requests write access for itself.

A PR opened or updated with `GITHUB_TOKEN` doesn't start CI on its own. GitHub creates the CI run with the status **Action required**, and a maintainer approves it. The version PR and branch appear as `github-actions[bot]`.

## Prepare a release

1. Make sure the changesets for the release are merged to `main`. To preview the result locally, run `knope prepare-release --dry-run` on an up-to-date `main`.
2. Open **Actions → Version → Run workflow**, select `main`, and run it. On any other branch, the workflow fails without changing anything.
3. If no changesets are pending, the run succeeds and its summary says so. No branch or PR changes.
4. Otherwise, the run creates or updates the PR titled `chore: prepare release vX.Y.Z` from the `knope/version` branch. The run summary links to it. Each run replaces that branch with `main` plus one new version commit.
5. Review the PR. Check the proposed version against the changes, the new `CHANGELOG.md` section, the version changes in `Cargo.toml`, `Cargo.lock`, and `crates/scidtopgn-cli/Cargo.toml`, and the deleted changesets.
6. To change the changelog wording, fix the changesets on `main` and run the workflow again. Commits pushed to `knope/version` by hand are lost on the next run.
7. Approve the CI run. On the PR, open the **Checks** tab, or find the CI run with the status **Action required** under **Actions**, and approve it. Each run of the Version workflow updates the branch and needs a new approval.
8. Wait for `ci-success` to pass, then merge the PR. Merging prepares the version; nothing is tagged or published.

If more changesets are merged to `main` while the version PR is open, run the workflow again to include them.

If the push is rejected with `refusing to allow a GitHub App to create or update workflow`, `main` has changed a file under `.github/workflows/` since the version branch was last pushed. `GITHUB_TOKEN` can't be given permission to change workflow files. Delete the `knope/version` branch on GitHub, which closes the version PR, and run the workflow again.

The first release, `0.1.0`, is already prepared in `Cargo.toml` and `CHANGELOG.md`, so it doesn't need a version PR.

## Updating Knope

Bump Knope deliberately. Update `KNOPE_VERSION` and `KNOPE_SHA256` in `version.yml`, and the version in this document and in `CONTRIBUTING.md`. The hash is the SHA-256 of `knope-x86_64-unknown-linux-musl.tgz`, which GitHub shows as the asset's digest on the release page. Read Knope's release notes, then check the bump table above with `knope prepare-release --dry-run` and a temporary changeset of each type.
