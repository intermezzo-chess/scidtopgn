# Releasing

Releases take two manual steps on `main`. The **Version** workflow opens a version PR that applies the pending changesets, and a maintainer reviews and merges it. The **Release** workflow then tags the prepared version, creates a GitHub release with binaries, and publishes both crates to crates.io, after a reviewer approves it. Merging to `main` never versions or publishes anything on its own.

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

### Version PRs

The Version workflow pushes the version branch and opens the PR with the workflow's own `GITHUB_TOKEN`. No personal token or secret is needed.

In the repository, open **Settings → Actions → General**. Under **Workflow permissions**, turn on **Allow GitHub Actions to create and approve pull requests**. Without it, the workflow fails at **Create or update the version PR**. Keep the default permissions read-only; the version job requests write access for itself.

A PR opened or updated with `GITHUB_TOKEN` doesn't start CI on its own. GitHub creates the CI run with the status **Action required**, and a maintainer approves it. The version PR and branch appear as `github-actions[bot]`.

### The `release` environment

The Release workflow's `release` job, which tags and publishes, runs in the `release` environment. Create it under **Settings → Environments → New environment**:

1. Name it `release`.
2. Turn on **Required reviewers** and add the maintainers who approve releases. Leave **Prevent self-review** off if one maintainer both starts and approves releases.
3. Under **Deployment branches and tags**, choose **Selected branches and tags** and add the branch rule `main`.
4. Add the `CARGO_REGISTRY_TOKEN` environment secret, described next.

Keep publishing credentials out of repository and organization secrets. Every job can read those; only the `release` job can read the environment's secrets.

### crates.io token

The first publish of a crate needs a crates.io API token.

1. Sign in to [crates.io](https://crates.io) with the GitHub account that will own the crates, and verify your email address. crates.io doesn't accept publishes without one.
2. Under **Account Settings → API Tokens**, create a token with an expiration date, the scopes **publish-new** and **publish-update**, and the crate scope `scidtopgn*`.
3. Save it in the `release` environment as `CARGO_REGISTRY_TOKEN`.

When the token expires, create a new one and replace the secret. Without a token, the `release` job fails before it tags anything.

### Switching to Trusted Publishing

Once both crates exist on crates.io, [Trusted Publishing](https://crates.io/docs/trusted-publishing) can replace the token. crates.io then issues a short-lived token to the workflow run.

1. On crates.io, open each crate's **Settings → Trusted Publishing** and add a GitHub publisher with owner `intermezzo-chess`, repository `scidtopgn`, workflow `release.yml`, and environment `release`.
2. In `release.yml`, add `id-token: write` to the `release` job's permissions. Before **Publish to crates.io**, add a step with `id: auth` that runs [`rust-lang/crates-io-auth-action`](https://github.com/rust-lang/crates-io-auth-action), pinned to a commit SHA. In **Publish to crates.io**, set `CARGO_REGISTRY_TOKEN` to `${{ steps.auth.outputs.token }}`. Remove the **Check the crates.io token** step.
3. After a release publishes this way, delete the `CARGO_REGISTRY_TOKEN` secret and revoke the token on crates.io.

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

## Publish a release

1. Open **Actions → Release → Run workflow**, select `main`, and run it. The run releases the version in `Cargo.toml` at the commit it started on. Pushes to `main` during the run don't change that.
2. Before anything waits for approval, the run:
   - fails on any branch other than `main`, when changesets are pending, when `CHANGELOG.md` has no section for the version, when the tag already points to another commit, or when the version is already fully released
   - runs CI on the commit
   - builds the `scidtopgn` binary for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`, each on a runner that can run it, and checks that `scidtopgn --version` prints the version and that the archive's `THIRD_PARTY_LICENSES` covers the binary's crates
   - runs `cargo publish --dry-run` for the crates that aren't on crates.io yet
3. The **check** job's summary shows the version, the commit, the release notes, and which release steps are already done. Once every other job passes, the `release` job waits. Open the run, choose **Review deployments**, select `release`, and approve it. To stop instead, reject it. Nothing has been tagged or published at that point.
4. After approval, the `release` job:
   1. Creates the `vX.Y.Z` tag on the run's commit.
   2. Creates a draft GitHub release with the version's `CHANGELOG.md` section as its notes. It attaches the archives `scidtopgn-vX.Y.Z-<target>.tar.gz`, or `.zip` for Windows, and a `SHA256SUMS` file, checks every archive against `SHA256SUMS`, and then publishes the release.
   3. Publishes `scidtopgn`, waits until the crates.io index serves it, and then publishes `scidtopgn-cli`. crates.io comes last because a published version can't be deleted.
5. [Verify the release](#verify-a-release).

Each archive contains a `scidtopgn-vX.Y.Z-<target>` directory with the binary, `LICENSE`, `THIRD_PARTY_LICENSES`, and `README.md`.

## Third-party notices

`THIRD_PARTY_LICENSES` holds the license notices of the crates compiled into the `scidtopgn` binary. The MIT License requires that its copyright and permission notice go with every copy, including binaries. The file is maintained by hand. Review it whenever a dependency is added, removed, or changes version in `Cargo.lock`, and before every release.

1. List the crates in each target's binary. Proc macros run only at build time, so they're left out:

   ```sh
   for target in x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc; do
     cargo tree --locked -p scidtopgn-cli --target "$target" -e normal,no-proc-macro --prefix none --format '{p} {l}'
   done | sort -u
   ```

2. For each crate other than `scidtopgn` and `scidtopgn-cli`, list `<name> <version>` at the start of a line, with the targets and the license used. Copy its MIT license file unchanged from the crate as published, such as from `~/.cargo/registry/src/index.crates.io-*/<name>-<version>/`. When a crate offers MIT or Apache-2.0, use MIT. A crate under any other license, or with a `NOTICE` file, needs a maintainer decision before it's released.
3. Remove crates that are no longer in any binary.

The Release workflow's **Check the archive's notices** step fails, before anything waits for approval, when the archive lacks a crate's listing or the full text of its MIT license file. Run the same check locally with `.github/scripts/check-archive-notices`; the step in `release.yml` shows how.

## Recover from a partial failure

Each step of the `release` job skips what is already done: a tag on the run's commit, archives that finished uploading to the release, and crate versions already on crates.io. An upload that failed partway, which GitHub leaves as an asset in the `starter` state, is deleted and uploaded again. To recover, fix the cause and run the job again. The workflow refuses only when the tag, the published release with `SHA256SUMS`, and both crates all exist.

- **A job before approval fails.** Nothing has been tagged or published. Fix the cause on `main` and start a new run. For a transient failure, rerun the failed jobs instead.
- **The `release` job fails.** Fix the cause, such as a missing or expired token. Then open the run and choose **Re-run jobs → Re-run failed jobs**. The rerun uses the same commit and the same archives, and needs approval again. GitHub allows reruns for 30 days.
- **You start a new run instead.** That works until the tag exists. After that, a new run refuses if `main` has moved, because the tag points to an older commit; rerun the original run instead. A new run builds new archives. Archives already attached to the release are kept, and `SHA256SUMS` is computed from the archives the release has.
- **The original run can't be rerun anymore** and the tag exists. Finish by hand from the tag. For each crate missing from crates.io, in order, run `cargo publish --locked -p <crate>` on a checkout of `vX.Y.Z`. Fix the GitHub release in the web interface.
- **The job reports that `SHA256SUMS` and the archives don't match.** An asset was changed or deleted by hand. Delete `SHA256SUMS` and any wrong archive from the release, then rerun. Until the check passes, a new release stays a draft.
- **A published crate is broken.** crates.io doesn't allow replacing or deleting a version. Yank it with `cargo yank --version X.Y.Z <crate>` and release a fix as a new version.

## Prereleases

To publish a prerelease such as `0.1.1-rc.0`, run **Version** with a **Prerelease label**, such as `rc`. A label is lowercase letters and digits and starts with a letter. Merge the version PR and run **Release** as usual. The GitHub release is marked as a prerelease. Cargo only installs a prerelease when asked for it, for example with `cargo install scidtopgn-cli --version 0.1.1-rc.0 --locked`.

Checked against Knope 0.23.0:

- The prerelease is the next stable version with `-<label>.N` added. From `0.1.0` with a `minor` changeset pending, it's `0.1.1-rc.0`.
- Knope keeps the changesets after a prerelease. Each prerelease section in `CHANGELOG.md` lists every change since the last stable release.
- Running **Version** again with the same label prepares the next number, such as `0.1.1-rc.1`, even without new changesets.
- For a prerelease, **Release** treats only changesets added or changed since the commit that set the version as pending.

To return to stable, run **Version** with the label empty. Knope prepares the stable version, such as `0.1.1`, with a changelog section that collects all changes since the last stable release, and deletes the changesets.

## Verify a release

After the run succeeds, check each part. Replace `X.Y.Z` with the version.

1. The tag points to the commit in the **check** job's summary:

   ```sh
   git fetch --tags
   git rev-parse 'vX.Y.Z^{commit}'
   ```

2. The release has four archives and `SHA256SUMS`, and the checksums match:

   ```sh
   gh release download vX.Y.Z --repo intermezzo-chess/scidtopgn --dir release-check
   cd release-check
   sha256sum --check SHA256SUMS   # on macOS: shasum -a 256 --check SHA256SUMS
   ```

3. crates.io has both crates at the version. Open [scidtopgn](https://crates.io/crates/scidtopgn) and [scidtopgn-cli](https://crates.io/crates/scidtopgn-cli), or, with Cargo 1.82 or later, run:

   ```sh
   cargo info scidtopgn@X.Y.Z
   cargo info scidtopgn-cli@X.Y.Z
   ```

4. Optionally, install it with the minimum supported Rust version, and run it:

   ```sh
   cargo +1.70 install scidtopgn-cli --version X.Y.Z --locked
   scidtopgn --version
   ```

## Updating Knope

Bump Knope deliberately. Update `KNOPE_VERSION` and `KNOPE_SHA256` in `version.yml`, and the version in this document and in `CONTRIBUTING.md`. The hash is the SHA-256 of `knope-x86_64-unknown-linux-musl.tgz`, which GitHub shows as the asset's digest on the release page. Read Knope's release notes, then check the bump table above with `knope prepare-release --dry-run` and a temporary changeset of each type.
