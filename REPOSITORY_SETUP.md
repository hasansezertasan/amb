# Repository setup and releasing

Releases are fully automatic from Conventional Commits; nobody tags by hand.

1. Merge PRs to `main` with Conventional Commit titles (`feat:`, `fix:`, ...).
2. release-please keeps one release PR open, bumping `Cargo.toml` and `Cargo.lock` and writing `CHANGELOG.md`.
3. Merging that PR starts `.github/workflows/release.yml`, which:
   - creates the GitHub release as a **draft** and its tag (`force-tag-creation`),
   - builds native binaries for x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-msvc, attests their provenance, and generates a CycloneDX SBOM,
   - publishes the crate to crates.io,
   - uploads every asset to the draft,
   - publishes the release last.

Publishing last matters: with release immutability enabled, a published release can no longer have its tag or assets changed.

## Assets

`amb-<rust-target>.tar.gz` (`.zip` on Windows) with a `.sha256` next to each, plus `amb-sbom.cdx.json`.
Each archive holds a single `amb-<rust-target>/` directory with the binary, so mise's `github:` backend installs it with no extra configuration.
Linux builds are glibc-only, built natively on their own runners, so system libraries link normally.

## One-time settings

The settings below are not files, so they cannot be reviewed in a pull request. Do them before merging the first release PR.

Each step is tagged:

- **[AGENT]**: a `gh` command that can run unattended. Replace nothing: `OWNER/REPO` below is `hasansezertasan/amb`.
- **[HUMAN]**: browser-only work.
- **[CHECK]**: a scriptable verification, or an explicit "no scriptable check".

## Squash-only merges

The PR title becomes the commit and the changelog line, so merge by squash only.

**[AGENT]**

```sh
gh api -X PATCH repos/hasansezertasan/amb \
  -F allow_squash_merge=true -F allow_merge_commit=false -F allow_rebase_merge=false \
  -F delete_branch_on_merge=true
```

**[CHECK]**

```sh
gh api repos/hasansezertasan/amb \
  --jq '.allow_squash_merge and (.allow_merge_commit | not) and (.allow_rebase_merge | not)' | grep -qx true
```

## Let Actions open the release PR

release-please runs as a GitHub Action and opens the release pull request.

**[AGENT]**

```sh
gh api -X PUT repos/hasansezertasan/amb/actions/permissions/workflow \
  -F default_workflow_permissions=read \
  -F can_approve_pull_request_reviews=true
```

**[CHECK]**

```sh
gh api repos/hasansezertasan/amb/actions/permissions/workflow \
  --jq '.can_approve_pull_request_reviews == true and .default_workflow_permissions == "read"' | grep -qx true
```

## Release immutability

Once published, a release's tag and assets can no longer be changed. The release workflow publishes last
so this works.

**[HUMAN]** Settings, General, Releases: enable release immutability. It only applies to releases published afterwards.

**[CHECK]**

```sh
gh api repos/hasansezertasan/amb/immutable-releases --jq '.enabled' | grep -qx true
```

## The `publish` environment

Attestation minting and crates.io publishing run in an environment that only `main` may deploy to. `workflow_dispatch` runs the workflow file from the selected branch, so this policy, which GitHub evaluates outside the file, is what contains it.

**[AGENT]**

```sh
gh api -X PUT repos/hasansezertasan/amb/environments/publish \
  --input - <<'JSON'
{ "deployment_branch_policy": { "protected_branches": false, "custom_branch_policies": true } }
JSON
gh api -X POST repos/hasansezertasan/amb/environments/publish/deployment-branch-policies \
  -f name=main -f type=branch
```

**[CHECK]**

```sh
gh api repos/hasansezertasan/amb/environments/publish/deployment-branch-policies \
  --jq '[.branch_policies[].name] == ["main"]' | grep -qx true
```
## crates.io and trusted publishing

The workflow publishes with a short-lived token obtained through trusted publishing, so no token is stored
in GitHub. The `amb` crate already exists (0.1.0 and 0.1.1 were published with an API token), so the
first-publish steps of the template do not apply here.

**[HUMAN]** On the crate's page, Settings, Trusted Publishing, add a GitHub publisher with owner `hasansezertasan`, repository `amb`, workflow filename `release.yml` and environment `publish`. The environment must match the `environment: publish` of the publish job. This is already configured.

**[CHECK]**

```sh
curl -s -A "setup-check" https://crates.io/api/v1/crates/amb | jq -e '.crate.name == "amb"'
```

The trusted publisher itself has no scriptable check: the first release after the switch is its test. If the
publish job fails before uploading, fix the setting and re-run the failed jobs.

**Moving a repository from token-based publishing.** Changing the workflow does not delete the
`CARGO_REGISTRY_TOKEN` secret or revoke the token. After the first release that publishes through trusted
publishing succeeds, remove it. The secret may sit in the `publish` environment or, if it was added at
repository level, in the repository secrets, which jobs using the environment can read as well. Remove both.

**[AGENT]**

```sh
gh secret delete CARGO_REGISTRY_TOKEN --env publish --repo hasansezertasan/amb || true
gh secret delete CARGO_REGISTRY_TOKEN --repo hasansezertasan/amb || true
```

**[HUMAN]** Revoke the token at https://crates.io/settings/tokens, so no long-lived publish credential stays
available to jobs using the environment.

**[CHECK]**

```sh
[ "$(gh secret list --env publish --repo hasansezertasan/amb --json name \
  --jq 'map(select(.name == "CARGO_REGISTRY_TOKEN")) | length')" = 0 ]
[ "$(gh secret list --repo hasansezertasan/amb --json name \
  --jq 'map(select(.name == "CARGO_REGISTRY_TOKEN")) | length')" = 0 ]
```

**[HUMAN]** Only if the repository belongs to an organization and the token was an organization secret: remove
this repository from that secret's access list in the organization's settings (`gh secret list --org ORG`
shows it) instead of deleting the secret, which other repositories may share. The checks above cannot see it:
GitHub's organization-secrets endpoint for a repository rejects user-owned repositories.

A failing `gh` (no authentication, no access) prints nothing, so the comparison fails instead of reporting the
secret as gone. The deletion commands above ignore errors on purpose, since the secret usually exists in only
one scope; this check is what proves the result.
## Branch ruleset for `main`

`.github/rulesets/main.json` requires a pull request, linear history, squash merges, resolved review threads
and the CI checks (including the dependency review and the package check), and blocks force pushes and deletion.
The dependency review fails without GitHub's dependency graph, so enable it (see "Dependency graph" below)
before applying the ruleset. Repository admins may bypass through a pull request,
which is how the release PR is merged: it is opened with `GITHUB_TOKEN`, so the required checks never run on it.

**[AGENT]**

```sh
gh api -X POST repos/hasansezertasan/amb/rulesets --input .github/rulesets/main.json
```

**[CHECK]**

```sh
gh api repos/hasansezertasan/amb/rulesets --jq '[.[].name] | index("main") != null' | grep -qx true
```
## Managed `.gitignore`

The generated `.gitignore` only ignores `/target`. [cobo](https://github.com/hasansezertasan/cobo) replaces it
with the upstream [github/gitignore](https://github.com/github/gitignore) boilerplates for Rust, the three
operating systems and the common editors, inside a sealed fence, and records the upstream commit in `cobo.lock`
so drift can be detected. Add your own entries below the end fence; a hand edit inside the fence breaks its seal.

**[AGENT]**

```sh
uvx --python 3.14 --from cobo==0.6.0 cobo update
uvx --python 3.14 --from cobo==0.6.0 cobo gitignore dump Rust macOS Windows Linux VisualStudioCode JetBrains \
  --out .gitignore --lock --eol lf
git add .gitignore cobo.lock
```

**[CHECK]**

```sh
uvx --python 3.14 --from cobo==0.6.0 cobo check --strict
```

To refresh later, run `uvx --python 3.14 --from cobo==0.6.0 cobo update` and then the same command with `sync`. cobo 0.6.0 needs Python 3.14, which `uvx` fetches if it is missing.

## Dependency graph

`dependency-review.yml` fails a pull request that adds a vulnerable dependency. It reads GitHub's dependency
graph.

**[HUMAN]** Settings, Advanced Security: enable the dependency graph.

**[CHECK]**

```sh
gh api --silent repos/hasansezertasan/amb/dependency-graph/compare/main...main
```

## Renovate

**[HUMAN]** Install the Renovate GitHub app on the repository. `.github/renovate.json` extends the shared preset.

**[CHECK]** No scriptable check: Renovate opens its onboarding or dependency dashboard issue after installation.

## Known limitations

- A pull request opened with `GITHUB_TOKEN` (the release PR) does not trigger other workflows, so required status checks stay pending on it. Either do not require checks for `release-please--*` branches or merge it as a repository admin.
- release-please's "Created releases" comment links the draft's transient URL, which 404s after publication. The release itself is correct.
- If a release run fails after the draft is created, do not delete the tag once the release is published: an immutable release's tag can never be reused. A failed run before publication leaves only a draft; delete the draft and its tag, and revert the release commit's version bump so release-please proposes the same version again.
