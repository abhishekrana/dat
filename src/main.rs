//! CLI: open a file or stdin in the reader, or render it inline to stdout.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Parser, ValueEnum};
use tracing::{error, info};

use dat::app::App;
use dat::buffer::Buffer;
use dat::config::{self, Config, ConfigError};
use dat::layout::Layouter;
use dat::style::{self, StyleError};
use dat::theme::Theme;
use dat::{doc, render};

/// Width for --inline when neither a flag, fzf nor a terminal says otherwise.
const DEFAULT_INLINE_WIDTH: u16 = 120;
/// fzf exports its preview pane's width here.
const FZF_WIDTH_ENV: &str = "FZF_PREVIEW_COLUMNS";
/// The style when neither a flag nor the config names one.
const DEFAULT_STYLE: &str = "github";

/// A markdown reader for the terminal that reads like a page.
#[derive(Debug, Parser)]
#[command(name = "dat", version, about)]
struct Args {
    /// Markdown file; stdin when absent and not a terminal.
    file: Option<PathBuf>,
    /// Render to stdout instead of opening the reader.
    #[arg(long)]
    inline: bool,
    /// Output for --inline: auto is ansi on a terminal and plain in a pipe.
    #[arg(long, value_enum, default_value_t = Format::Auto, requires = "inline")]
    format: Format,
    /// Style name: a built-in or a file in ~/.config/dat/styles [default: github].
    #[arg(long)]
    style: Option<String>,
    /// Theme flavor from the palette [default: solarized-light].
    #[arg(long, env = "DAT_THEME")]
    theme: Option<String>,
    /// Config file [default: ~/.config/dat/config.toml].
    #[arg(long, env = "DAT_CONFIG")]
    config: Option<PathBuf>,
    /// Pane width for --inline; defaults to fzf's preview width, else the terminal's, else 120.
    #[arg(long)]
    width: Option<u16>,
    /// List the styles that can be loaded and exit.
    #[arg(long)]
    list_styles: bool,
    /// Do not follow the file on disk (the reader reloads on change by default).
    #[arg(long)]
    no_watch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Auto,
    Ansi,
    Plain,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let to_stderr = args.inline || args.list_styles;
    let _guard = match dat::log::init(to_stderr) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("dat: cannot open the log: {e}");
            None
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !to_stderr {
                error!(error = format!("{e:#}"), "exit");
            }
            eprintln!("dat: {e:#}");
            let file_error = e.downcast_ref::<StyleError>().is_some() || e.downcast_ref::<ConfigError>().is_some();
            ExitCode::from(if file_error { 2 } else { 1 })
        }
    }
}

fn run(args: &Args) -> anyhow::Result<()> {
    if args.list_styles {
        list_styles();
        return Ok(());
    }
    let config_path = args.config.clone().or_else(config::path);
    let config = match &config_path {
        Some(path) => config::load(path)?,
        None => Config::default(),
    };
    let theme = match (&args.theme, &config.theme) {
        (Some(id), _) => Theme::by_id(id)?,
        (None, Some(id)) => Theme::by_id(id).with_context(|| {
            let shown = config_path
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            format!("config {shown}")
        })?,
        (None, None) => Theme::default_theme()?,
    };
    let style_name = args
        .style
        .as_deref()
        .or(config.style.as_deref())
        .unwrap_or(DEFAULT_STYLE);
    let style = style::load(style_name, style::user_styles_dir().as_deref())?;
    let buffer = match &args.file {
        Some(path) => Buffer::from_path(path)?,
        None if io::stdin().is_terminal() => bail!("no file given and stdin is a terminal (see --help)"),
        None => Buffer::from_reader(io::stdin().lock())?,
    };
    info!(path = ?buffer.path(), bytes = buffer.len_bytes(), style = %style.name, theme = %theme.id, "loaded");

    if !args.inline {
        if !io::stdout().is_terminal() {
            bail!("stdout is not a terminal; use --inline to render into a pipe");
        }
        return App::new(buffer, style, theme)
            .with_watch(!args.no_watch && config.watch.unwrap_or(true))
            .run()
            .context("terminal");
    }
    let document = doc::parse(&buffer);
    let tty = io::stdout().is_terminal();
    let terminal_cols = tty.then(|| crossterm::terminal::size().ok().map(|(w, _)| w)).flatten();
    let width = inline_width(args.width, std::env::var(FZF_WIDTH_ENV).ok().as_deref(), terminal_cols);
    let page = Layouter::new(theme).layout(&document, &style, width);
    let ansi = match args.format {
        Format::Ansi => true,
        Format::Plain => false,
        Format::Auto => tty,
    };
    let text = if ansi {
        render::ansi(&page, theme, tty)
    } else {
        render::plain(&page)
    };
    io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .context("writing to stdout")
}

/// The flag wins, then fzf's preview width, then the terminal, then the default.
fn inline_width(flag: Option<u16>, fzf: Option<&str>, terminal: Option<u16>) -> u16 {
    flag.or_else(|| fzf.and_then(|v| v.trim().parse().ok()))
        .or(terminal)
        .filter(|w| *w > 0)
        .unwrap_or(DEFAULT_INLINE_WIDTH)
}

/// Built-ins, then user styles; a user file that shadows a built-in says so.
fn list_styles() {
    let mut user: Vec<String> = Vec::new();
    if let Some(dir) = style::user_styles_dir()
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                user.push(stem.to_owned());
            }
        }
    }
    user.sort();
    for name in style::builtins() {
        let shadowed = user.iter().any(|u| u == name);
        println!("{name}{}", if shadowed { "  (replaced by the user file)" } else { "" });
    }
    for name in user.iter().filter(|u| !style::builtins().any(|b| b == u.as_str())) {
        println!("{name}  (user)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_width_prefers_flag_then_fzf_then_terminal() {
        assert_eq!(inline_width(Some(72), Some("90"), Some(200)), 72);
        assert_eq!(inline_width(None, Some("90"), Some(200)), 90);
        assert_eq!(inline_width(None, Some("nope"), Some(200)), 200);
        assert_eq!(inline_width(None, None, None), DEFAULT_INLINE_WIDTH);
        assert_eq!(inline_width(None, Some("0"), None), DEFAULT_INLINE_WIDTH);
    }
}
