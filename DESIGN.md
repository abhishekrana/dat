# dat - design

_A markdown reader for the terminal that reads like a page in a browser, not like a terminal tool._

This is the spec for the first version. It says what dat is, how it is built, and what a style file looks like.
Everything in it was decided against four look mockups - the GitHub one was chosen.

## What it is

- A viewer for one markdown file, or stdin, rendered to a reading column with the typography of a browser-rendered page:
  rhythm, rules, tinted code, quiet tables, links in one accent.
- **Styles are files.** A look is a TOML file of per-element rules. GitHub is the first built-in; adding a look is
  adding a file, and the engine knows nothing about any of them.
- **Colours are roles, never hexes.** A style names `accent`, `surface`, `muted`; the flavor from
  `src/theme/palette.toml` says what those are. Every style works in every flavor, and `DAT_THEME` picks the flavor.
- Built to become an editor later without a rewrite: a rope holds the text, every node carries its byte span, and every
  rendered line knows the source range it came from. Nothing in v1 edits.

Not a browser (no history stack), not a file manager, not a notes app.

## Scope

**v1 - the reader**

- CommonMark plus GFM: tables, task lists, strikethrough, autolinks, footnotes, alerts (`> [!NOTE]`), front matter, math
  delimiters left as text, `==mark==` ignored.
- Vault syntax: `[[wikilinks]]` (with `|alias` and `#heading`), `#tags`. Both render; a wikilink opens the note if the
  vault holds one file of that name, else says so.
- Code blocks highlighted with bat's grammars and themes (two-face), language label from the fence.
- Links: OSC 8 on every link in `--inline`; a click follows one in the reader. `.md` targets open in dat, `#anchors`
  scroll, anything else goes to `xdg-open`.
- Outline (`t`) as an overlay, heading jumps, in-document search (`/`, `n`, `N`), help (`?`).
- Watch and reload, `--inline` to stdout for fzf and yazi previews, stdin.
- One built-in style (`github`), user styles from `~/.config/dat/styles/`, `--style`, `S` cycles at runtime.
- Theme by role: `--theme <flavor>`, `DAT_THEME`, default `solarized-light`.

**v2 - beyond the grid**

- Images via the Kitty protocol with unicode placeholders and tmux passthrough; halfblocks as fallback.
- Mermaid rendered as an image; math as an image with a Unicode fallback.
- Larger headings when Ghostty renders the text sizing protocol (1.3 parses it, does not draw it; tmux drops it).
- Reading position memory, a file picker.

**Never in the viewer**: editing UI. Editing is a separate mode built on the same model when it is wanted.

## Architecture

Five layers, one direction of data, no layer reaching past its neighbour:

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
| CLI      | `main`   | clap: args, `--inline`, stdin, exit codes. Thin.                                      |

The two facts that make the editor possible later are already in the table: the rope is the only text, and a `Line`
carries the span it was laid out from. Hit-testing a click is a lookup, not a search.

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

comrak is configured with `sourcepos` and the GFM, footnotes, front matter, alerts and wikilinks extensions. Tags are
not a comrak extension; a small pass over `Inline::Text` splits `#word` out, skipping headings and code.

### Layout

- **Measure.** The style sets `measure` (GitHub: 80) and `align` (`left`, the default: a 2-cell gutter; or `center`). A
  pane narrower than the measure forces the column to its width minus 2 cells each side. The outline rail, when a style
  turns it on, takes its width from the left first.
- **Rhythm.** A style says how many blank rows precede each element; the engine never inserts its own.
- **Wrapping** is per grapheme cluster using `unicode-width`, greedy, with a `Cell` per column so wide characters and
  emoji occupy two. Code never wraps: long lines are clipped with `→` in the last cell and pan with `h`/`l`.
- **Tables** get fair-share column widths. A table wider than the measure shrinks its widest column first, and a cell
  wraps inside its column, so a row grows taller rather than losing text. No modal in v1.
- **Inline styling is per span**, so a link, a code span or an emphasis is styled on exactly its cells, and every cell
  keeps the id of the inline it came from.
- **Relayout** happens on resize and on reload only, all blocks, cached by `(block hash, width)`. At document scale this
  is milliseconds; measured before anything smarter is added.

### Style files

A style is TOML. Keys are element names; values are rules. Colours are palette roles. Unknown keys fail loudly, so a
typo cannot silently fall back to a default.

```toml
# ~/.config/dat/styles/github.toml (the built-in, verbatim)
name    = "GitHub"
extends = "base"  # every built-in inherits base; a user style may extend any built-in
measure = "full"  # or a cell count, e.g. 80
align   = "left"  # or "center", for a fixed measure
rail    = false   # the outline rail is a layout switch, not an element

h1 = { fg = "emphasis", bold = true, rule = "column", above = 2, below = 1 }
h2 = { fg = "emphasis", bold = true, rule = "column", above = 1, below = 1 }
h3 = { fg = "emphasis", bold = true, rule = "none", above = 1, below = 1 }
h4 = { fg = "emphasis", bold = true, rule = "none", above = 1, below = 0 }

paragraph = { fg = "fg", below = 1 }
strong    = { fg = "emphasis", bold = true }
emph      = { italic = true }
strike    = { fg = "muted", strike = true }

link      = { fg = "accent", underline = true }
wikilink  = { fg = "accent", underline = true }
tag       = { fg = "muted" }
code_span = { fg = "changes", pad = 0 }

code    = { bg = "surface", pad = 1, label = "none", above = 0, below = 1 }
quote   = { bar = "▌", bar_fg = "muted", fg = "muted" }

list = { bullet = "•", bullet_fg = "muted", nested = "◦", indent = 2 }
task = { done = "󰄲", todo = "󰄱", done_fg = "done", todo_fg = "muted", done_text = "muted" }

table        = { lines = "box", header_bold = true, header_rule = true, line_fg = "border" }
rule         = { glyph = "─", fg = "border", width = "column" }
front_matter = { as = "table" }
footnote     = { marker = "superscript", fg = "accent", text_fg = "muted" }
image        = { placeholder = "󰥶", fg = "muted" }  # v1 draws "󰥶 alt (640×360)" on one line

[callout]
bar = "▌"
bar_fg = "accent"
title_fg = "accent"
icon = true
kinds = { note = "accent", tip = "done", important = "changes", warning = "asking", caution = "blocked" }
icons = { note = "󰋽", tip = "󰌶", important = "󰨄", warning = "󰀪", caution = "󰳦" }
```

Rule fields are a closed set per element, checked at load. `rule` on headings is `none | words | column`; `lines` on
tables is `none | rules | box`; `label` on code is `none | above | right`; `front_matter.as` is
`hidden | line | list | table`. That vocabulary is exactly the difference between the four mockups, so Air, Minimal and
Docs are each a file of overrides on `base`.

Resolution order: `base` (compiled in), then the named style's `extends` chain, then the style itself.
`~/.config/dat/styles/` is searched before the built-ins, so a user file named `github.toml` replaces the built-in.

### Theme

`src/theme/mod.rs` reads `src/theme/palette.toml` with `include_str!`, so a flavor is defined once and compiled in - no
codegen step. Roles available to styles:
`bg surface selection border fg emphasis muted accent changes float working asking blocked done`. Syntax highlighting
uses two-face's Solarized and Catppuccin themes, mapped per flavor, so a code block matches bat in the pane beside it.

## Keys

vi and less, nothing to learn:

| Key                   | Action                                                    |
| --------------------- | --------------------------------------------------------- |
| `j` `k` `↓` `↑`       | scroll a line; the wheel does the same                    |
| `d` `u` `PgDn` `PgUp` | half page; `space` a page                                 |
| `g` `G`               | top, bottom                                               |
| `]` `[`               | next, previous heading                                    |
| `t`                   | outline overlay; `↵` jumps, `Esc` closes                  |
| `/` `n` `N`           | search, next, previous; `Esc` clears                      |
| click                 | follow a link                                             |
| drag                  | select text; the release copies it (via `clip`)           |
| `Backspace`           | back to the note a link was followed from                 |
| `y`                   | copy the selection, else the code block at the top screen |
| `e`                   | open the file at the top line in `$VISUAL` or `$EDITOR`   |
| `r` `w`               | reload; toggle following the file on disk (on by default) |
| `T`                   | cycle theme through the palette's flavors                 |
| `S`                   | cycle style (deferred: one built-in style for now)        |
| `?` `q`               | help; quit                                                |

Following a link: `#anchor` scrolls to the heading, `[[Note]]` opens the note beside this file or anywhere under the
vault root (the nearest `.obsidian` or `.git`), a `.md` path opens relative to this file, anything else goes to
`xdg-open`. The TUI cannot emit OSC 8 (ratatui has no hyperlink cells), so links there are followed by clicking;
`--inline` on a terminal still wraps links in OSC 8.

Selection is dat's own, not the terminal's: mouse capture is on for the wheel and for click-to-follow, so the terminal
never sees the drag. A press anchors, a drag paints the range in `selection`, and a release copies through `clip` -
matching Ghostty's `copy-on-select`. A press that never moves is a click, so following a link is unchanged. What is
copied is the rendered page, not the source: wrapped lines are wrapped and trailing padding is trimmed per row.

The status line is one row on `surface`: file name, current section, percent, and three hints. It is the viewer's,
inside the pane; tmux keeps its own below.

## CLI

```
dat [FILE]                  # a file, or stdin when FILE is absent and stdin is not a TTY
dat --inline [FILE]         # render to stdout, no TUI; width: --width, else FZF_PREVIEW_COLUMNS, else the
                            # terminal, else 120 - so fzf and yazi previews fit without flags
dat --inline --format plain # force plain or ansi; auto (default) is ansi on a terminal, plain in a pipe
dat --style github --theme solarized-dark --no-watch FILE
dat --list-styles
```

Exit codes: 0, 1 on a bad argument or unreadable file, 2 on a style file that fails to load (named, with the key).

## Outside dat

dat starts three programs and assumes nothing else about the machine:

- `clip` - copying (selection release, `y`) pipes the text to a `clip` command on `PATH`; without one, the copy reports
  that it failed.
- `$VISUAL` or `$EDITOR` - `e` opens the file at the top line.
- `xdg-open` - a link that is neither an anchor, a wikilink nor a `.md` file.

## Engineering conventions

Modern, boring Rust; the community's defaults, not ours.

- Edition 2024, MSRV as `rust-version` in `Cargo.toml`, `Cargo.lock` committed. rustfmt defaults except
  `max_width = 120`. clippy at `-D warnings` with `clippy::pedantic` enabled and a short, justified allow-list in
  `Cargo.toml`.
- `thiserror` in library modules, `anyhow` only in `main`. No `unwrap` outside tests. No `unsafe`.
- One crate, `src/` modules as in the table above; a module is one job. `lib.rs` exposes what `main.rs` and the tests
  use, nothing else.
- Prefer a maintained crate over our own: ratatui, crossterm, comrak, ropey, syntect + two-face, unicode-width, clap
  (derive), notify (watch), serde + toml, insta (snapshots).
- **Tests are snapshots.** The mockups' sample note is the fixture; each built-in style has an `--inline` snapshot at 80
  and 60 columns, so a rendering change is a reviewed diff. Wrapping, width and table allocation have unit tests.
  Nothing tests the live terminal.
- Comments say what, one line. History goes in commit messages.

## Open questions

- Where a wikilink resolves when the file is outside a vault: the file's directory, then nothing. Good enough for v1.
- Whether `--inline` should emit OSC 8 (yes for a terminal, no for a pipe; detect the TTY).
- The Docs look's rail width and whether the rail scrolls with the page or stays fixed. Decided when that style is
  written, not now.
