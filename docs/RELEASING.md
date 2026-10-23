# Releasing

Releases are fully automatic from Conventional Commits; nobody tags by hand.

1. Merge PRs to `main` with Conventional Commit titles (`feat:`, `fix:`, ...).
2. release-please keeps one release PR open, bumping `Cargo.toml` and `Cargo.lock` and writing `CHANGELOG.md`.
3. Merging that PR starts `.github/workflows/release.yml`, which:
   1. creates the GitHub release as a **draft** and its tag (`force-tag-creation`),
   2. builds native binaries for Linux (x86_64, aarch64), macOS (Intel, Apple silicon) and Windows, attests their provenance, and generates a CycloneDX SBOM,
   3. publishes the crate to crates.io,
   4. uploads every asset to the draft,
   5. publishes the release last.

Publishing last matters: with release immutability enabled, a published release can no longer have its tag or assets changed.

## Assets

`amb-<rust-target>.tar.gz` (`.zip` on Windows) with a `.sha256` next to each, plus `amb-sbom.cdx.json`.
Each archive holds a single `amb-<rust-target>/` directory with the binary, so mise's `github:` backend installs it with no extra configuration.
Linux builds are glibc-only: cpal links ALSA dynamically, so musl is not an option.

## One-time repository setup

These are settings, not files, so they cannot be reviewed in a PR.

- **Release immutability**: Settings, General, Releases, enable release immutability. It only applies to releases published afterwards.
- **Actions may open PRs**: Settings, Actions, General, Workflow permissions, enable "Allow GitHub Actions to create and approve pull requests".
- **`publish` environment**: Settings, Environments, create `publish`. Restrict deployment branches to `main` (the workflow relies on this, since `workflow_dispatch` runs the file from the selected branch).
- **crates.io trusted publishing**: on the crate's page, Settings, Trusted Publishing, add a GitHub publisher with owner `hasansezertasan`, repository `amb`, workflow filename `release.yml` and environment `publish`. The environment must match the `environment: publish` of the publish job. No token is stored: the job exchanges its OIDC token for a short-lived crates.io token.
- **First publish of a new crate**: trusted publishing only works for a crate that already exists, so the very first publish needs an API token (https://crates.io/settings/tokens, scopes `publish-new` and `publish-update`) added as the `CARGO_REGISTRY_TOKEN` secret of the `publish` environment, with the publish step temporarily reading that secret instead of the auth action's token. `amb` is already published, so this no longer applies to it.
- **Renovate**: install the Renovate GitHub app on the repository; `.github/renovate.json` extends the shared preset.

## Known limitations

- A pull request opened with `GITHUB_TOKEN` (the release PR) does not trigger other workflows, so required status checks stay pending on it. Either do not require checks for `release-please--*` branches or close and reopen the PR to run them.
- release-please's "Created releases" comment links the draft's transient URL, which 404s after publication. The release itself is correct.
- If a release run fails after crates.io publishing, re-run only the failed jobs. Do not delete the tag: an immutable release's tag can never be reused.
