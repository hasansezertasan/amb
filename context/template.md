## Pinned to a copier-rust release tag

**Id:** bd13ce4c-a413-46af-a05c-8c50ef07c638
**Type:** decision
**Status:** active
**Evidence:** confirmed
**Source:** maintainer conversation, 2026-10-23; the maintainer's handoff note of the same day
**Verification:** uncorroborated
**Revisit when:** copier-rust changes its answers file or its tagging scheme

amb was brought onto the copier-rust template with `copier copy --overwrite`.
The first run (PR #10) was made without a tag, because the template had no
release yet, and recorded `_commit: 21e76fb`. The maintainer then asked for a
version instead, so `.copier-answers.yml` now records `v0.1.2`, and the
adoption PR was re-pinned as each template release appeared (v0.1.0, v0.1.1,
v0.1.2) before it merged.

**Reason:** the maintainer's handoff note says the adoption needs a tag to pin
to, and that a bare commit hash is not something to leave in the answers file.
Why `copier update` cannot follow a hash was not tested here.

**Rejected alternative:** wait for the template's first release before
adopting at all. The maintainer chose to adopt unpinned and re-pin.

## The main ruleset requires every CI job, including package check and dependency review

**Id:** 04e36280-3b5b-421f-b8eb-6195b18af3ee
**Type:** decision
**Type:** constraint
**Status:** active
**Evidence:** confirmed
**Source:** maintainer conversation, 2026-10-23; review findings on PR #10; copier-rust PR #9
**Verification:** corroborated
**Revisit when:** a CI job is added to or removed from `ci.yml`, or the release PR starts running CI

`.github/rulesets/main.json` requires nine checks: `Format and clippy`, the
three `Test (...)` jobs, `MSRV check`, `Package check`, `Dependency review`,
`Validate PR title` and `Validate branch name`. Repository admins can bypass
through a pull request, which is how the release PR is merged: it is opened
with `GITHUB_TOKEN`, so no required check ever runs on it.

`Dependency review` fails without GitHub's dependency graph, so the graph has
to be enabled before the ruleset is applied.

**Reason:** the template's first ruleset left out `Package check` and
`Dependency review`, so a failing `cargo publish --dry-run` or a newly
introduced vulnerable dependency could still merge. Two review findings on the
adoption PR pointed that out.

**Rejected alternative:** add the two checks to amb's `main.json` only. Rejected
by the maintainer in favour of fixing the template first (copier-rust#9) and
pinning to the release that carries it, so amb does not drift from the template
and conflict on the next `copier update`.

## docs/RELEASING.md is gone; the release notes live in REPOSITORY_SETUP.md

**Id:** 0a85b883-b8d6-40e8-9d54-69fb085adbda
**Type:** decision
**Status:** active
**Evidence:** confirmed
**Source:** maintainer conversation, 2026-10-23; review findings on copier-rust PR #3 and PR #4
**Verification:** uncorroborated
**Revisit when:** copier-rust ships a `docs/RELEASING.md` again

copier-rust folded `docs/RELEASING.md` into `REPOSITORY_SETUP.md` and no longer
renders the file; amb's own copy, which only added the trusted-publishing
migration notes on top of it, is removed in PR #12 now that the migration is
finished (until that merges, the file is still in the tree). The crates.io section of amb's
`REPOSITORY_SETUP.md` also differs from the template's: it drops the
first-publish steps, because the `amb` crate already existed when the file was
generated.

**Reason:** one document for how releases work and the one-time settings, and
a template that has no release yet has no older renders that still link to the
removed file.

**Rejected alternative:** keep `docs/RELEASING.md` as a short pointer to
`REPOSITORY_SETUP.md`. A review finding argued that projects generated from an
earlier template keep a link to it; the maintainer chose to delete the file
because no release of the template existed yet.

**Consequence:** the amb-specific crates.io section can conflict on the next
`copier update`. Adding a "crate already published" question to the template
was not pursued; whether it is worth it was never decided.
