# Changelog

Notable changes, in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) spirit and versioned with
[SemVer](https://semver.org/).

## [0.1.0] - 2026-10-07

### Added

- A config file for the theme, style and watch (1cc86ae)
- **folio**: Select text with the mouse (9d2be54)
- **folio**: Live reload, reading positions, trace edges (412bc11)
- **folio**: Navigation - outline, search, link hints and clicks, back, copy, edit, help (c8ec0b8)
- **folio**: T cycles the theme, logs rotate daily (e1cfb2e)
- **folio**: --inline width follows fzf, then the terminal, default 120 (d582d84)
- **folio**: Nerd Font callout icons and image placeholder, icons live in the style (6e2cc7b)
- **folio**: Nerd Font checkboxes for tasks in the GitHub style (ed38b48)
- **folio**: Syntax highlighting in code blocks (5a82ac5)
- **folio**: No zebra tint on table rows in the GitHub style (436ff66)
- **folio**: Inline code is colour alone, no tint and no padding (f2eb6c1)
- **folio**: No blank row after a ruled heading in the GitHub style (6b113d5)
- **folio**: The column fills the pane; a fixed measure stays a style option (dbe72b8)
- **folio**: The column starts at the left, and align is a style switch (3697351)
- **folio**: A markdown reader that reads like a page (f639251)

### Build

- Prettier in the gate, quiet cargo-deny warnings (f5113a7)
- The gate, CI and the tag-driven release (e962c47)

### Changed

- Block spacing names every block kind (7d6ef80)
- The library exports only what the binary and tests use (23265ca)
- **breaking** - Drop style keys and palette entries nothing reads (cf261a4)
- **breaking** - Rename folio to dat and stand alone (91e9d10)
- **folio**: Drop link hints; a click follows a link (8c02ff6)

### Documentation

- Plain wording in the docs, --help and style errors (99ee69d)
- DESIGN.md describes the reader that exists (3d89b6b)
- README, and CLAUDE.md and DESIGN.md for a standalone repo (816b980)
- **folio**: Correct DESIGN.md's Makefile, toolchain and mockup notes (a7164f0)
- **folio**: The theme is compiled in, not generated (90d55b3)
- **folio**: Fzf previews markdown through folio --inline (1c5f51b)

### Fixed

- **breaking** - One rule for exit codes (f5ba247)
- A user style can extend the built-in it replaces (6f395fd)
- Punctuation after a link wraps with it (caa0b9f)
- Copying, the editor and the terminal hold up outside the dotfiles (0fb9c20)
- Two readers no longer erase each other's reading positions (20a0615)
- Rewriting the open file in place no longer sends the reader to the top (273949a)
- Wikilink headings land, and a link carries its kind (79c3923)
- The help overlay wraps instead of clipping in a narrow pane (9d4bed7)
- **folio**: Table cells wrap inside their column instead of clipping (806cb3d)
- **folio**: List depth saturates, and the reader refuses a stdout that is not a terminal (3e27568)
- **folio**: List depth, footnote numbers, and a row can never exceed the measure (45b9cde)

### Tests

- A run that exits before reading stdin no longer fails the CLI test (e91b137)
- The config lookup end to end, and the gate checks the stated conventions (057d032)
- **folio**: Keep the hostile-input literal within 120 columns (129c644)
- **folio**: Hardening and CLI contract tests (4104e77)
- **folio**: An edge-case fixture and reader scrolling tests (a2cbbc8)

