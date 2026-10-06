<div align="center">

# dat

**Markdown in the terminal, read like a page.** `cat` prints a file, `bat` colours it, `dat` lays markdown out the way a
browser would: rhythm, rules, tinted code, quiet tables and one accent colour for links.

[![ci](https://github.com/abhishekrana/dat/actions/workflows/ci.yml/badge.svg)](https://github.com/abhishekrana/dat/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/abhishekrana/dat)](https://github.com/abhishekrana/dat/releases)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
![rust](https://img.shields.io/badge/rust-%E2%89%A5%201.98-orange)

[Install](#install) · [Usage](#usage) · [Keys](#keys) · [Styles and themes](#styles-and-themes) ·
[Development](#development)

</div>

## Install

From a release (Linux x86_64):

```sh
v=0.1.0
curl -LO "https://github.com/abhishekrana/dat/releases/download/v$v/dat-$v-x86_64-unknown-linux-gnu.tar.gz"
tar xzf "dat-$v-x86_64-unknown-linux-gnu.tar.gz"
install "dat-$v-x86_64-unknown-linux-gnu/dat" ~/.local/bin/
```

From source, with Rust 1.98 or newer:

```sh
cargo install --locked --git https://github.com/abhishekrana/dat
```

Checkboxes and callout icons are Nerd Font glyphs.

## Usage

```sh
dat README.md                       # the reader: scroll, search, follow links
dat --inline README.md              # render to stdout and exit
some-command | dat --inline         # stdin works too
dat --theme solarized-dark FILE     # or set DAT_THEME
```

`--inline` writes ANSI on a terminal and plain text in a pipe; `--format ansi|plain` forces one. Its width is `--width`,
else fzf's preview width, else the terminal's, else 120, so it fits an fzf preview with no flags:

```sh
fzf --preview 'dat --inline --format ansi {}'
```

The reader follows the file on disk and reloads when it changes (`--no-watch` turns that off), and remembers where you
were in each file.

Exit codes: `0`, `1` on a bad argument or unreadable file, `2` on a config or style file that fails to load.

## Keys

vi and less, nothing to learn:

| Key                   | Action                                                  |
| --------------------- | ------------------------------------------------------- |
| `j` `k` `↓` `↑`       | scroll a line; the wheel does the same                  |
| `d` `u` `PgDn` `PgUp` | half page; `space` a page                               |
| `g` `G`               | top, bottom                                             |
| `]` `[`               | next, previous heading                                  |
| `t`                   | outline; `↵` jumps, `Esc` closes                        |
| `/` `n` `N`           | search, next, previous; `Esc` clears                    |
| click                 | follow a link                                           |
| drag                  | select text; the release copies it                      |
| `Backspace`           | back to the note a link was followed from               |
| `y`                   | copy the selection, else the code block at the top      |
| `e`                   | open the file at the top line in `$VISUAL` or `$EDITOR` |
| `r` `w`               | reload; toggle following the file on disk               |
| `T`                   | cycle the theme                                         |
| `?` `q`               | help; quit                                              |

A link to `#anchor` scrolls to the heading, `[[Note]]` opens that note from the same vault, a `.md` path opens in dat,
and anything else goes to `xdg-open`. Copying pipes the text to a `clip` command on `PATH`.

## Styles and themes

A **style** is the look: a TOML file with one table per element. `github` is built in; files in `~/.config/dat/styles/`
are loaded by name with `--style NAME` and replace a built-in of the same name. `dat --list-styles` shows what is
available. A style names colour roles (`accent`, `surface`, `muted`, ...), never hex values, so every style works in
every theme.

A **theme** gives the roles their colours: `solarized-light` (default), `solarized-dark`, `catppuccin-latte`,
`catppuccin-mocha`. Code blocks are highlighted with bat's grammars and matching themes.

[DESIGN.md](DESIGN.md) has the style file format and every element it can set.

## Configuration

dat reads `~/.config/dat/config.toml` (`$XDG_CONFIG_HOME/dat/config.toml`) when it exists. Every key is optional:

```toml
theme = "solarized-dark"  # default: solarized-light
style = "github"          # a built-in or a file in ~/.config/dat/styles/
watch = true              # follow the file on disk and reload on change
```

A flag beats an environment variable, which beats the config file, which beats the built-in default. An unknown key is
an error, so a typo is reported rather than ignored.

| Variable          | Effect                                                                     |
| ----------------- | -------------------------------------------------------------------------- |
| `DAT_THEME`       | theme, as `--theme`                                                        |
| `DAT_CONFIG`      | config file to read instead, as `--config`                                 |
| `DAT_LOG`         | log level (`debug`, `info`, ...); the reader logs to `~/.local/state/dat/` |
| `XDG_CONFIG_HOME` | where `dat/config.toml` and `dat/styles/` are looked up                    |
| `XDG_STATE_HOME`  | where logs and reading positions are kept                                  |

## Development

```sh
cargo test                       # unit tests and insta snapshots
scripts/install-dev-tools.sh     # the pinned lint toolchain, cargo-deny, git-cliff
scripts/check.sh                 # the gate CI runs
```

A changed rendering fails the snapshot tests; accept it with `INSTA_UPDATE=always cargo test`, then read the snapshot
diff before committing. Commits follow [Conventional Commits](https://www.conventionalcommits.org/), which
`CHANGELOG.md` and the release notes are generated from.

## License

[MIT](LICENSE)
