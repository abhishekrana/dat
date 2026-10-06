# CLAUDE.md

dat: a markdown reader for the terminal that reads like a page. One Rust crate, one binary; ratatui, comrak. `DESIGN.md`
is the spec - read it before changing what anything looks like. `README.md` is the user's view.

## Commands

The loop, cheapest first. Run only as far as the change warrants.

```sh
cargo check
cargo test                                   # unit tests and insta snapshots
INSTA_UPDATE=always cargo test               # accept changed snapshots, then read the diff before committing
cargo run -- tests/fixtures/sample.md        # the reader
cargo run -- --inline tests/fixtures/sample.md   # to stdout: ANSI on a terminal, plain in a pipe
DAT_LOG=debug cargo run -- FILE              # log to ~/.local/state/dat/dat.<date>.log, 3 days kept
scripts/check.sh                             # the gate, before committing
```

`scripts/check.sh` is the gate: CI runs the same script, so a clean local run means a clean CI run. Missing optional
tools are reported as skipped. `scripts/install-dev-tools.sh` installs the pinned ones from `scripts/versions.env`.

## Two rules

- **Styles are files.** A look is `styles/<name>.toml`: one table per element, closed field sets, `extends` for
  composition. Nothing in `src/` knows what GitHub looks like. A new look is a new file; a rendering change that is not
  expressible in a style file is a schema change in `src/style/mod.rs` and `styles/base.toml` together.
- **Colours are roles.** Styles name `accent`, `surface`, `muted`; `src/theme` maps roles to hexes from
  `src/theme/palette.toml`, which is `include_str!`'d so a flavor is defined once. Never a hex in `src/` or a style.

## Layout of the code

`buffer` (rope) -> `doc` (blocks and inlines, every node with a byte `Span`) -> `layout` (rows of `Segment`s at a
measure, cached per block and width) -> `render` (ratatui or ANSI/plain) with `app` running the Elm loop and `ui`
drawing. `main.rs` is the CLI edge and the only place `anyhow` appears.

## Conventions

- Pinned everything: `=x.y.z` in `Cargo.toml`, `Cargo.lock` committed, builds `--locked`. Bumps are their own commits.
- Toolchains, deliberately distinct: `rust-version` in `Cargo.toml` is the MSRV, checked by the gate; `RUST_VERSION` in
  `scripts/versions.env` is what fmt and clippy run under, so a new clippy is adopted on purpose; builds and tests use
  the installed stable. There is no `rust-toolchain.toml`.
- `[lints]` in `Cargo.toml`: pedantic clippy, no `unwrap`/`expect` outside tests (`clippy.toml`), no `unsafe`.
- Errors are `thiserror` enums per module that name the file and key; `main` maps a `StyleError` to exit 2, anything
  else to 1.
- Tests: unit tests next to the code, CLI and hardening tests in `tests/`, snapshots in `tests/snapshots/` from
  `tests/fixtures/`. A snapshot diff is a reviewed rendering change, never a rubber stamp.
- `deny.toml`: permissive licences only; an ignored advisory carries its reason.
- Line width ≤120 everywhere: `rustfmt.toml` for Rust, `.prettierrc` for markdown, yaml and json.
- Comments state what the code does now, one line. History belongs in the commit message.
- Conventional Commits: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build` reach `CHANGELOG.md`; `ci` and
  `chore(release)` are filtered out (`cliff.toml`).

## Rules

- **Self-sufficient.** No server, script or runtime the user has to install. The only programs dat starts are `clip`,
  `$VISUAL`/`$EDITOR` and `xdg-open`, each on a user action. It assumes no particular terminal, theme or dotfiles.
- **Nothing personal or work-related in this repository.** The public identity is the whole of it: the owner name in
  `LICENSE` and the GitHub handle in URLs and the commit identity. No employer, colleagues, hostnames, home directory
  paths or credentials - in code, comments, fixtures and commit messages alike. The git identity is repo-local; CI runs
  gitleaks over the tree and the full history.

## Cutting a release

Run in order, stop at the first failure.

1. **Preconditions.** `git status --porcelain` empty, `git fetch && git rev-list --count origin/main..main` is `0`.
2. **Gate.** `scripts/check.sh` passes with **nothing skipped**.
3. **CI green on `HEAD`.** Local success is not evidence; a cancelled run is not a pass.
   ```sh
   curl -s "https://api.github.com/repos/abhishekrana/dat/actions/runs?head_sha=$(git rev-parse HEAD)" \
     | jq -r '.workflow_runs[] | "\(.name) \(.status)/\(.conclusion)"'
   ```
4. **Version.** `git tag --list 'v*' --sort=-v:refname | head -1` is the previous one. SemVer; on `0.x` a breaking
   change bumps the minor.
5. **Prepare.** `scripts/release.sh vX.Y.Z` sets the version in `Cargo.toml` and regenerates `CHANGELOG.md`.
6. **Review.** `git diff` - the changelog names every user-visible change.
7. **Commit and push.** `git commit -am "chore(release): vX.Y.Z" && git push`
8. **CI green on the release commit.** Repeat step 3.
9. **Tag.** `git tag -a vX.Y.Z -m "dat X.Y.Z" && git push origin vX.Y.Z`
10. **Watch.** The tag triggers `release.yml`, which re-runs the gate, builds `--locked`, and publishes the tarball, its
    checksum, a provenance attestation and the git-cliff notes.
11. **Verify.** `gh release view vX.Y.Z`, and `gh attestation verify <tarball> --repo abhishekrana/dat`.

On failure, fix forward and cut the next patch. **A published tag is never moved or deleted.**
