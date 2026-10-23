# Contributing

Thanks for helping with amb.

## Workflow

1. Branch from `main` with a Conventional Branch name: `feature/...`, `bugfix/...`, `chore/...`.
2. Make the change and run `mise run check` (or `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`).
3. Open a pull request. The title must follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `ci:`, `chore:` and so on. It becomes the squash commit and the changelog entry, so describe the effect of the change.

## Releases

Releases are automatic; see [REPOSITORY_SETUP.md](REPOSITORY_SETUP.md). Do not tag by hand.
