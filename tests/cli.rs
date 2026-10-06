//! The binary's contract: exit codes, stdin, output modes, and the messages a caller sees.
#![allow(clippy::expect_used)]

use std::io::Write as _;
use std::process::{Command, Output, Stdio};

fn dat(args: &[&str], stdin: Option<&str>) -> Output {
    dat_env(args, stdin, &[])
}

/// Runs the binary with no `DAT_THEME` and no config file unless `env` sets them.
fn dat_env(args: &[&str], stdin: Option<&str>, env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dat"));
    cmd.env_remove("DAT_THEME")
        .env("DAT_CONFIG", "/no/such/dat/config.toml");
    cmd.envs(env.iter().copied());
    cmd.args(args)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("dat starts");
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(text.as_bytes()).expect("write stdin");
    }
    child.wait_with_output().expect("dat exits")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_missing_file_exits_one_with_the_path() {
    let out = dat(&["--inline", "/no/such/note.md"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("/no/such/note.md"), "{}", stderr(&out));
}

#[test]
fn an_unknown_style_exits_two_and_lists_the_builtins() {
    let out = dat(&["--inline", "--style", "nope", "tests/fixtures/sample.md"], None);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("github"), "{}", stderr(&out));
}

#[test]
fn an_unknown_theme_exits_one_and_lists_the_flavors() {
    let out = dat(&["--inline", "--theme", "mocha", "tests/fixtures/sample.md"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("catppuccin-mocha"), "{}", stderr(&out));
}

#[test]
fn stdin_renders_plain_in_a_pipe_and_ansi_when_asked() {
    let plain = dat(&["--inline", "--width", "40"], Some("# Hi\n\nSome *text*.\n"));
    assert!(plain.status.success());
    let text = String::from_utf8_lossy(&plain.stdout);
    assert!(text.contains("Hi") && !text.contains('\u{1b}'), "{text}");
    let ansi = dat(&["--inline", "--format", "ansi", "--width", "40"], Some("# Hi\n"));
    assert!(
        String::from_utf8_lossy(&ansi.stdout).contains("\u{1b}[1;38;2;"),
        "bold coloured heading"
    );
}

#[test]
fn format_requires_inline_and_a_terminal_less_run_without_a_file_fails() {
    let out = dat(&["--format", "plain", "tests/fixtures/sample.md"], None);
    assert_eq!(out.status.code(), Some(2), "clap usage error");
    let out = dat(&[], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("not a terminal"), "{}", stderr(&out));
    let out = dat(&["--inline"], None);
    assert!(out.status.success(), "an empty pipe is an empty document, not an error");
}

#[test]
fn list_styles_and_help_are_available() {
    let out = dat(&["--list-styles"], None);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && text.contains("github") && text.contains("base"));
    let help = dat(&["--help"], None);
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(
        text.contains("--inline") && text.contains("--no-watch") && text.contains("DAT_THEME"),
        "{text}"
    );
}

fn config_file(name: &str, text: &str) -> String {
    let dir = std::env::temp_dir().join(format!("dat-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    std::fs::write(&path, text).expect("write config");
    path.to_string_lossy().into_owned()
}

fn ansi(args: &[&str], env: &[(&str, &str)]) -> String {
    let mut all = vec!["--inline", "--format", "ansi", "--width", "60"];
    all.extend_from_slice(args);
    let out = dat_env(&all, Some("# Hi\n\nSome *text* and a [link](x.md).\n"), env);
    assert!(out.status.success(), "{}", stderr(&out));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn the_config_sets_the_theme_and_the_env_and_flag_override_it() {
    let dark = config_file("theme", "theme = \"solarized-dark\"\n");
    let from_config = ansi(&[], &[("DAT_CONFIG", &dark)]);
    assert_eq!(from_config, ansi(&["--theme", "solarized-dark"], &[]));
    assert_ne!(from_config, ansi(&[], &[]), "the default is solarized-light");
    let mocha = ansi(&["--theme", "catppuccin-mocha"], &[]);
    assert_eq!(
        ansi(&[], &[("DAT_CONFIG", &dark), ("DAT_THEME", "catppuccin-mocha")]),
        mocha
    );
    assert_eq!(ansi(&["--config", &dark, "--theme", "catppuccin-mocha"], &[]), mocha);
}

#[test]
fn a_bad_config_exits_two_naming_the_file_and_key() {
    let path = config_file("bad", "colour = \"blue\"\n");
    let out = dat_env(&["--inline"], Some("# Hi\n"), &[("DAT_CONFIG", &path)]);
    assert_eq!(out.status.code(), Some(2));
    let err = stderr(&out);
    assert!(err.contains("config.toml") && err.contains("colour"), "{err}");
}

#[test]
fn a_config_style_that_does_not_exist_is_a_style_error() {
    let path = config_file("style", "style = \"nope\"\n");
    let out = dat_env(&["--inline"], Some("# Hi\n"), &[("DAT_CONFIG", &path)]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("nope"), "{}", stderr(&out));
}

#[test]
fn an_unknown_theme_in_the_config_names_the_config() {
    let path = config_file("badtheme", "theme = \"mocha\"\n");
    let out = dat_env(&["--inline"], Some("# Hi\n"), &[("DAT_CONFIG", &path)]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("config.toml") && err.contains("catppuccin-mocha"), "{err}");
}
