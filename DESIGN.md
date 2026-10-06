# dat - design

_A markdown reader for the terminal that lays a file out the way a browser renders a page._

This is the spec for the first version. It says what dat is, how it is built and what a style file looks like. Each
decision in it came from comparing four mockups of the look; dat implements the GitHub one.

## What it is

- A viewer for one markdown file, or stdin, rendered to a reading column with the typography of a browser-rendered page:
  spacing between blocks, rules, code on a tinted background, tables ruled in the border colour, links in one accent.
- **Styles are files.** A look is a TOML file of per-element rules. GitHub is the first built-in. Adding a look means
  adding a file, and no engine code is specific to any look.
- **Colours are roles, never hexes.** A style names `accent`, `surface`, `muted`; the flavor from
  `src/theme/palette.toml` sets their colours. Every style works in every flavor, and `DAT_THEME` picks the flavor.
- Built to become an editor later without a rewrite: a rope holds the text, every node carries its byte span, and every
  rendered line records the source range it came from. Nothing in v1 edits.

dat is not a browser (it has no history stack), a file manager or a notes app.

## Scope

**v1 - the reader**

- CommonMark plus GFM: tables, task lists, strikethrough, autolinks, footnotes, alerts (`> [!NOTE]`), front matter, math
  delimiters left as text, `==mark==` ignored.
- Vault syntax: `[[wikilinks]]` (with `|alias` and `#heading`), `#tags`. Both render. A wikilink opens the note if the
  vault holds one file of that name; otherwise dat says so.
- Code blocks highlighted with bat's grammars and themes (two-face), with the language label taken from the fence.
- Links: `--inline` wraps every link in OSC 8, and a click follows one in the reader. `.md` targets open in dat,
  `#anchors` scroll, and `xdg-open` opens anything else.
- Outline (`t`) as an overlay, heading jumps, in-document search (`/`, `n`, `N`), help (`?`).
- Watch and reload, `--inline` to stdout for fzf and yazi previews, stdin.
- One built-in style (`github`), user styles from `~/.config/dat/styles/`, `--style`.
- Theme by role: `--theme <flavor>`, `DAT_THEME`, default `solarized-light`.
- `~/.config/dat/config.toml` sets `theme`, `style` and `watch`; an environment variable overrides it, and a flag
  overrides both.

**v2 - beyond the grid**

- Images via the Kitty protocol with unicode placeholders and tmux passthrough; halfblocks as fallback.
- Mermaid rendered as an image; math as an image with a Unicode fallback.
- Larger headings when Ghostty renders the text sizing protocol (Ghostty 1.3 parses it but does not draw it, and tmux
  drops it).
- A file picker.

The viewer never gets an editing UI. Editing, when it is wanted, is a separate mode built on the same model.

## Architecture

There are five layers. Data flows in one direction, and no layer reaches past its neighbour:

```
source (rope) -> parse (comrak) -> Document -> layout (measure) -> Lines -> paint (ratatui) -> screen
                                       ^                              |
                                       +--- style: rules per element --+
```

| Layer    | Module   | Owns                                                                                  |
| -------- | -------- | ------------------------------------------------------------------------------------- |
| Buffer   | `buffer` | `ropey::Rope`; the only copy of the text. v1 loads and reloads; edits come later.     |
| Document | `doc`    | `Block` tree with `Span` (byte range) on every block and inline. Built from comrak.   |
| Style    | `style`  | Loads TOML into `Style`: one `Rule` per element, resolved against defaults/`extends`. |
| Layout   | `layout` | `Document` × `Style` × width → `Vec<Line>`; each `Line` has cells, and a `Span`.      |
| Theme    | `theme`  | Role → colour for a flavor. Compiled in from `palette.toml`.                          |
| Paint    | `ui`     | ratatui widgets: the page, outline overlay, search bar, help, status line.            |
| App      | `app`    | Elm loop: `Model`, `Msg`, `update`, `view`. Keys, mouse, watch events, mode.          |
| CLI      | `main`   | clap: args, `--inline`, stdin, exit codes. Holds little logic.                        |

Two facts in the table make a later editor possible: the rope is the only copy of the text, and each `Line` carries the
span it was laid out from. Hit-testing a click is therefore a lookup, not a search.

### Document model

```rust
pub struct Span { pub start: usize, pub end: usize }          // byte offsets into the rope

pub enum Block {
    FrontMatter { fields: Vec<(String, String)>, span },
    Heading { level: u8, inlines: Vec<Inline>, id: String, span },
    Paragraph { inlines, span },
    List { ordered: bool, items: Vec<ListItem>, span },        // ListItem { task: Option<bool>, blocks }
    Quote { blocks: Vec<Block>, span },
    Callout { kind: CalloutKind, title: Option<String>, blocks, span },
    Code { lang: Option<String>, text: String, span },
    Table { align: Vec<Align>, head: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>>, span },
    Rule { span },
    Footnote { label: String, blocks, span },
    Image { alt: String, src: String, span },                  // v1 draws a placeholder line
}

pub enum Inline {
    Text(String, Span), Code(String, Span), Strong(Vec<Inline>, Span), Emph(Vec<Inline>, Span),
    Strike(Vec<Inline>, Span), Link { text: Vec<Inline>, href: String, span },
    WikiLink { target: String, alias: Option<String>, span }, Tag(String, Span),
    FootnoteRef(String, Span), SoftBreak, HardBreak,
}
```

dat configures comrak with `sourcepos` and the GFM, footnotes, front matter, alerts and wikilinks extensions. Tags are
not a comrak extension, so a small pass over `Inline::Text` splits `#word` out and skips headings and code.

### Layout

- **Measure.** The style sets `measure` and `align`. `measure` is `full` (the pane's width, which GitHub uses) or a cell
  count. `align` is `left` (the default, with a 2-cell gutter) or `center`. In a pane narrower than the measure, the
  column shrinks to the pane's width minus 2 cells on each side.
- **Rhythm.** A style says how many blank rows precede each element; the engine never inserts its own.
- **Wrapping** is greedy and per grapheme cluster, measured with `unicode-width`. Each column is one `Cell`, so wide
  characters and emoji take two. Code never wraps; a long line is clipped with `→` in the last cell.
- **Tables** get fair-share column widths. A table wider than the measure shrinks its widest column first, and a cell
  wraps inside its column, so a row grows taller instead of losing text. v1 has no modal view for tables.
- **Inline styling is per span.** A link, a code span or an emphasis styles only its own cells, and every cell keeps the
  id of the inline it came from.
- **Relayout** happens only on resize, reload and theme change. It lays out every block, cached by
  `(block hash, width)`. For one document this takes milliseconds; measure before adding anything smarter.

### Style files

A style is TOML. Keys are element names; values are rules. Colours are palette roles. An unknown key is a load error, so
a typo cannot fall back to a default unnoticed.

`styles/base.toml` sets every element; a style names only what it changes. This is the built-in GitHub style, verbatim:

```toml
# The rendering everyone already reads: rules across the column under H1 and H2, boxed tables, pills for code.
name = "GitHub"
extends = "base"

h1 = { rule = "column", below = 0 }
h2 = { rule = "column", below = 0 }

wikilink = { underline = true }

quote = { bar = "▌", bar_fg = "muted", fg = "muted" }
task = { done = "󰄲", todo = "󰄱" } # Nerd Font checkboxes (nf-md-checkbox_marked / _blank_outline)
table = { lines = "box" }
front_matter = { as = "table" }

[callout]
bar = "▌"
icon = true
```

Each element has a closed set of rule fields, checked at load. `rule` on headings is `none | words | column`; `lines` on
tables is `none | rules | box`; `label` on code is `none | above | right`; `front_matter.as` is
`hidden | line | list | table`. That vocabulary covers every difference between the four mockups, so Air, Minimal and
Docs can each be a file of overrides on `base`.

Resolution order: `base` (compiled in), then the named style's `extends` chain, then the style itself. dat searches
`~/.config/dat/styles/` before the built-ins, so a user file named `github.toml` replaces the built-in.

### Theme

`src/theme/mod.rs` reads `src/theme/palette.toml` with `include_str!`, so each flavor is defined once and compiled in
with no codegen step. The roles available to styles are
`bg surface selection border fg emphasis muted accent changes float working asking blocked done`. Syntax highlighting
uses two-face's Solarized and Catppuccin themes, mapped per flavor, so a code block matches bat in the pane beside it.

## Keys

The keys follow vi and less:

| Key                   | Action                                                         |
| --------------------- | -------------------------------------------------------------- |
| `j` `k` `↓` `↑`       | scroll a line; the wheel does the same                         |
| `d` `u` `PgDn` `PgUp` | half a page; `space` a full page                               |
| `g` `G`               | top, bottom                                                    |
| `]` `[`               | next, previous heading                                         |
| `t`                   | outline overlay; `↵` jumps, `Esc` closes                       |
| `/` `n` `N`           | search, next, previous; `Esc` clears                           |
| click                 | follow a link                                                  |
| drag                  | select text; releasing the button copies it                    |
| `Backspace`           | go back to the note you followed the link from                 |
| `y`                   | copy the selection, else the topmost code block on screen      |
| `e`                   | open the file at the top line in `$VISUAL` or `$EDITOR`        |
| `r` `w`               | reload; toggle reloading when the file changes (on by default) |
| `T`                   | cycle through the palette's flavors                            |
| `?` `q`               | help; quit                                                     |

When you follow a link, `#anchor` scrolls to the heading, `[[Note]]` opens the note beside this file or anywhere under
the vault root (the nearest `.obsidian` or `.git`), a `.md` path opens relative to this file, and `xdg-open` opens
anything else. The TUI cannot emit OSC 8 because ratatui has no hyperlink cells, so in the reader you follow a link by
clicking it. `--inline` on a terminal still wraps links in OSC 8.

dat handles selection itself, not the terminal. Mouse capture is on for the wheel and for click-to-follow, so the
terminal never receives the drag. A press sets the anchor, a drag highlights the range in the `selection` colour, and a
release copies it, as Ghostty's `copy-on-select` does. A press that never moves is a click, so it still follows a link.
dat copies the rendered page, not the source: wrapped lines stay wrapped, and trailing padding is trimmed on each row.

The status line is one row on the `surface` colour that shows the file name, current section, percent and three hints.
It belongs to the viewer and sits inside the pane; tmux's own status line stays below.

## CLI

```
dat [FILE]                  # a file, or stdin when FILE is absent and stdin is not a TTY
dat --inline [FILE]         # render to stdout, no TUI; width: --width, else FZF_PREVIEW_COLUMNS, else the
                            # terminal, else 120 - so fzf and yazi previews fit without flags
dat --inline --format plain # force plain or ansi; auto (default) is ansi on a terminal, plain in a pipe
dat --style github --theme solarized-dark --no-watch FILE
dat --list-styles
```

dat exits with 0 on success, 2 when the request is wrong (a flag, a theme or style name, a config or style file, which
the error names along with the key) and 1 when something fails while running, such as a file that cannot be read.

## Outside dat

dat starts three programs and assumes nothing else about the machine:

- A clipboard tool. Copying (selection release, `y`) pipes the text to `wl-copy` under Wayland, `xclip` under X11 or
  `pbcopy` on macOS. With none of them, dat writes OSC 52 and the terminal sets the clipboard, if it allows that.
- `$VISUAL` or `$EDITOR`. `e` runs it through `sh` to open the file at the top line, so a value with arguments
  (`code --wait`) works.
- `xdg-open`, for a link that is not an anchor, a wikilink or a `.md` file.

## Engineering conventions

dat uses current, conventional Rust and follows the community's defaults rather than its own.

- Edition 2024, MSRV as `rust-version` in `Cargo.toml`, `Cargo.lock` committed. rustfmt defaults except
  `max_width = 120`. clippy at `-D warnings` with `clippy::pedantic` enabled and a short, justified allow-list in
  `Cargo.toml`.
- `thiserror` in library modules, `anyhow` only in `main`. No `unwrap` outside tests. No `unsafe`.
- One crate, with `src/` modules as in the table above; each module does one job. `lib.rs` exposes what `main.rs` and
  the tests use, and nothing else.
- Prefer a maintained crate over our own: ratatui, crossterm, comrak, ropey, syntect + two-face, unicode-width, clap
  (derive), notify (watch), serde + toml, insta (snapshots).
- **Tests are snapshots.** The sample note is the fixture. The GitHub style has `--inline` snapshots of it at 100 and 64
  columns and one per theme, and an edge-case fixture has its own, so every rendering change shows up as a diff to
  review. Wrapping, width and table allocation have unit tests. No test drives the live terminal.
- A comment says what the code does, in one line. History goes in commit messages.

## Open questions

- Where a wikilink should resolve when the file is outside a vault. dat looks in the file's directory and nowhere else,
  which is good enough for v1.
