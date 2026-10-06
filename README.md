<div align="center">

# dat

**Markdown in the terminal, read like a page.** `cat` prints a file, `bat` colours it, and `dat` lays markdown out the
way a browser would, with spacing between blocks, rules, code on a tinted background, tables ruled in the border colour
and one accent colour for links.

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

`--inline` writes ANSI on a terminal and plain text in a pipe, and `--format ansi|plain` forces one. The width is
`--width`, else fzf's preview width, else the terminal's, else 120, so the output fits an fzf preview with no flags:

```sh
fzf --preview 'dat --inline --format ansi {}'
```

The reader reloads the file when it changes on disk (`--no-watch` turns that off) and remembers your position in each
file.

dat exits with `0` on success, `2` when the request is wrong (a flag, a theme or style name, a config or style file) and
`1` when something fails while running, such as a file that cannot be read.

## Keys

The keys follow vi and less:

| Key                   | Action                                                  |
| --------------------- | ------------------------------------------------------- |
| `j` `k` `↓` `↑`       | scroll a line; the wheel does the same                  |
| `d` `u` `PgDn` `PgUp` | half a page; `space` a full page                        |
| `g` `G`               | top, bottom                                             |
| `]` `[`               | next, previous heading                                  |
| `t`                   | outline; `↵` jumps, `Esc` closes                        |
| `/` `n` `N`           | search, next, previous; `Esc` clears                    |
| click                 | follow a link                                           |
| drag                  | select text; releasing the button copies it             |
| `Backspace`           | go back to the note you followed the link from          |
| `y`                   | copy the selection, else the code block at the top      |
| `e`                   | open the file at the top line in `$VISUAL` or `$EDITOR` |
| `r` `w`               | reload; toggle reloading when the file changes          |
| `T`                   | cycle the theme                                         |
| `?` `q`               | help; quit                                              |

A link to `#anchor` scrolls to the heading, `[[Note]]` opens that note from the same vault, a `.md` path opens in dat,
and `xdg-open` opens anything else. Copying uses `wl-copy` on Wayland, `xclip` on X11 or `pbcopy` on macOS. Without any
of them, dat copies through the terminal with OSC 52, which tmux passes on with `set -g set-clipboard on`.

## Styles and themes

A **style** sets the look. It is a TOML file with one table per element. `github` is built in, `--style NAME` loads a
file by name from `~/.config/dat/styles/`, and a file there replaces a built-in of the same name. `dat --list-styles`
lists the styles dat can load. A style names colour roles (`accent`, `surface`, `muted`, ...), never hex values, so
every style works in every theme.

A **theme** gives the roles their colours: `solarized-light` (default), `solarized-dark`, `catppuccin-latte`,
`catppuccin-mocha`. dat highlights code blocks with bat's grammars and a syntax theme that matches the dat theme.

[DESIGN.md](DESIGN.md) has the style file format and every element it can set.

## Configuration

dat reads `~/.config/dat/config.toml` (`$XDG_CONFIG_HOME/dat/config.toml`) when it exists. Every key is optional:

```toml
theme = "solarized-dark"  # default: solarized-light
style = "github"          # a built-in or a file in ~/.config/dat/styles/
watch = true              # follow the file on disk and reload on change
```

A flag overrides an environment variable, which overrides the config file, which overrides the built-in default. An
unknown key is an error, so dat reports a typo instead of ignoring it.

| Variable          | Effect                                                                     |
| ----------------- | -------------------------------------------------------------------------- |
| `DAT_THEME`       | theme, as `--theme`                                                        |
| `DAT_CONFIG`      | config file to read instead, as `--config`                                 |
| `DAT_LOG`         | log level (`debug`, `info`, ...); the reader logs to `~/.local/state/dat/` |
| `XDG_CONFIG_HOME` | directory dat looks in for `dat/config.toml` and `dat/styles/`             |
| `XDG_STATE_HOME`  | directory for logs and reading positions                                   |

## Development

```sh
cargo test                       # unit tests and insta snapshots
scripts/install-dev-tools.sh     # the pinned lint toolchain, cargo-deny, git-cliff
scripts/check.sh                 # the gate CI runs
```

A changed rendering fails the snapshot tests; accept it with `INSTA_UPDATE=always cargo test`, then read the snapshot
diff before committing. Commits follow [Conventional Commits](https://www.conventionalcommits.org/), and git-cliff
generates `CHANGELOG.md` and the release notes from them.

## License

[MIT](LICENSE)
