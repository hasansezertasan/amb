# Repository setup

One-time GitHub settings the release flow depends on. They are settings, not files, so they cannot be
reviewed in a pull request. Do them before merging the first release PR.

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
in GitHub. Trusted publishing only works for a crate that already exists, so create the crate first with a
placeholder release. The version in `Cargo.toml` starts at `0.0.0`, and release-please takes it to `0.1.0`.

**[HUMAN]** Verify the email address of your crates.io account (https://crates.io/settings/profile). crates.io refuses to publish without it.

**[HUMAN]** Check that the name `amb` is free: https://crates.io/crates/amb must show "not found".

**[HUMAN]** Create an API token at https://crates.io/settings/tokens with the `publish-new` scope, then publish the placeholder from your machine:

```sh
cargo login
cargo publish
```

Revoke that token afterwards: it was only for this first publish.

**[HUMAN]** On the crate's page, Settings, Trusted Publishing, add a GitHub publisher with owner `hasansezertasan`, repository `amb`, workflow filename `release.yml` and environment `publish`. The environment must match the `environment: publish` of the publish job.

**[CHECK]**

```sh
curl -s -A "setup-check" https://crates.io/api/v1/crates/amb | jq -e '.crate.name == "amb"'
```

The trusted publisher itself has no scriptable check: the first release is its test. If the publish job fails
before uploading, fix the setting and re-run the failed jobs.
## Branch ruleset for `main`

`.github/rulesets/main.json` requires a pull request, linear history, squash merges, resolved review threads
and the CI checks, and blocks force pushes and deletion. Repository admins may bypass through a pull request,
which is how the release PR is merged: it is opened with `GITHUB_TOKEN`, so the required checks never run on it.

**[AGENT]**

```sh
gh api -X POST repos/hasansezertasan/amb/rulesets --input .github/rulesets/main.json
```

**[CHECK]**

```sh
gh api repos/hasansezertasan/amb/rulesets --jq '[.[].name] | index("main") != null' | grep -qx true
```
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
