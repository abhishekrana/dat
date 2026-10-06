# CLAUDE.md

dat is a markdown reader for the terminal that lays a file out the way a browser renders a page. It is one Rust crate
with one binary, built on ratatui and comrak. `DESIGN.md` is the spec; read it before changing how anything looks.
`README.md` is the documentation for users.

## Commands

The commands run from cheapest to most expensive. Run only as many as the change needs.

```sh
cargo check
cargo test                                   # unit tests and insta snapshots
INSTA_UPDATE=always cargo test               # accept changed snapshots, then read the diff before committing
cargo run -- tests/fixtures/sample.md        # the reader
cargo run -- --inline tests/fixtures/sample.md   # to stdout: ANSI on a terminal, plain in a pipe
DAT_LOG=debug cargo run -- FILE              # log to ~/.local/state/dat/dat.<date>.log, 3 days kept
scripts/check.sh                             # the gate, before committing
```

`scripts/check.sh` is the gate, and CI runs the same script. The script reports missing optional tools as skipped.
`scripts/install-dev-tools.sh` installs the pinned ones from `scripts/versions.env`.

## Two rules

- **Styles are files.** A look is `styles/<name>.toml`, with one table per element, closed field sets and `extends` for
  composition. No code in `src/` is specific to the GitHub look. A new look is a new file. A rendering change that a
  style file cannot express needs a schema change in `src/style/mod.rs` and `styles/base.toml` together.
- **Colours are roles.** Styles name `accent`, `surface`, `muted`; `src/theme` maps roles to hexes from
  `src/theme/palette.toml`, which it compiles in with `include_str!` so each flavor is defined once. Never put a hex in
  `src/` or a style.

## Layout of the code

Data flows from `buffer` (the rope) to `doc` (blocks and inlines, every node with a byte `Span`), then to `layout` (rows
of `Segment`s at a measure, cached per block and width), then to `render` (ratatui, or ANSI or plain text). `app` runs
the Elm loop and `ui` draws. `main.rs` is the CLI layer and the only place `anyhow` appears. It resolves each setting
from the flag, then the environment, then `config.toml` (the `config` module), then the default. `paths` is the only
module that works out XDG directories.

## Conventions

- Everything is pinned: `=x.y.z` in `Cargo.toml`, `Cargo.lock` committed, builds `--locked`. Each bump is its own
  commit.
- The toolchains are kept distinct on purpose. `rust-version` in `Cargo.toml` is the MSRV, and the gate checks it. fmt
  and clippy run under `RUST_VERSION` from `scripts/versions.env`, so adopting a new clippy is a deliberate change.
  Builds and tests use the installed stable. There is no `rust-toolchain.toml`.
- `[lints]` in `Cargo.toml`: pedantic clippy, no `unwrap`/`expect` outside tests (`clippy.toml`), no `unsafe`.
- Each module has a `thiserror` error enum whose messages name the file and key. `main` exits 2 when the request is
  wrong (a usage, style, config or theme error) and 1 when something fails while running.
- Tests: unit tests next to the code, CLI and hardening tests in `tests/`, snapshots in `tests/snapshots/` from
  `tests/fixtures/`. A snapshot diff is a rendering change; review it before accepting it.
- `deny.toml` allows permissive licences only, and every ignored advisory states its reason.
- Lines are at most 120 columns everywhere: `rustfmt.toml` sets it for Rust, `.prettierrc` for markdown, yaml and json.
- Comments state what the code does now, one line. History belongs in the commit message.
- Commits follow Conventional Commits. `feat`, `fix`, `perf`, `refactor`, `docs`, `test` and `build` reach
  `CHANGELOG.md`; `cliff.toml` filters out `ci` and `chore(release)`.

## Rules

- **Self-sufficient.** dat needs no server, script or runtime that the user has to install. The only programs it starts
  are the session's clipboard tool, `$VISUAL`/`$EDITOR` and `xdg-open`, each only on a user action. It assumes no
  particular terminal, theme or dotfiles.
- **Nothing personal or work-related in this repository.** The only identity in it is the public one: the owner's name
  in `LICENSE`, and the GitHub handle in URLs and in the commit identity. Code, comments, fixtures and commit messages
  name no employer, colleagues, hostnames, home directory paths or credentials. The git identity is repo-local; CI runs
  gitleaks over the tree and the full history.

## Cutting a release

Run in order, stop at the first failure.

1. **Preconditions.** `git status --porcelain` prints nothing, and `git fetch && git rev-list --count origin/main..main`
   prints `0`.
2. **Gate.** `scripts/check.sh --strict` passes; it fails if any step was skipped.
3. **CI green on `HEAD`.** A local pass is not evidence, and a cancelled run is not a pass.
   ```sh
   curl -s "https://api.github.com/repos/abhishekrana/dat/actions/runs?head_sha=$(git rev-parse HEAD)" \
     | jq -r '.workflow_runs[] | "\(.name) \(.status)/\(.conclusion)"'
   ```
4. **Version.** `git tag --list 'v*' --sort=-v:refname | head -1` prints the previous version. Versions follow SemVer;
   on `0.x` a breaking change bumps the minor version.
5. **Prepare.** `scripts/release.sh vX.Y.Z` sets the version in `Cargo.toml` and regenerates `CHANGELOG.md`.
6. **Review.** Run `git diff` and check that the changelog names every user-visible change.
7. **Commit and push.** `git commit -am "chore(release): vX.Y.Z" && git push`
8. **CI green on the release commit.** Repeat step 3.
9. **Tag.** `git tag -a vX.Y.Z -m "dat X.Y.Z" && git push origin vX.Y.Z`
10. **Watch.** The tag triggers `release.yml`, which re-runs the gate, builds `--locked`, and publishes the tarball, its
    checksum, a provenance attestation and the git-cliff notes.
11. **Verify.** `gh release view vX.Y.Z`, and `gh attestation verify <tarball> --repo abhishekrana/dat`.

On failure, fix forward and cut the next patch. **Never move or delete a published tag.**
