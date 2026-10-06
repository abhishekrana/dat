//! Style loading: user directory first, then built-ins; `extends` chains deep-merge onto their parent.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::Style;

/// Built-in styles, compiled into the binary.
const BUILTIN: &[(&str, &str)] = &[
    ("base", include_str!("../../styles/base.toml")),
    ("github", include_str!("../../styles/github.toml")),
];

const MAX_CHAIN: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum StyleError {
    #[error("style {name:?} not found (looked in {dir} and the built-ins: {})", builtin_names())]
    NotFound { name: String, dir: String },
    #[error("style {name:?}: {err}")]
    Parse { name: String, err: Box<toml::de::Error> },
    #[error("style {name:?}: cannot read {path}: {err}")]
    Read {
        name: String,
        path: PathBuf,
        err: std::io::Error,
    },
    #[error("style {name:?}: the `extends` chain is longer than {MAX_CHAIN}, which may mean a cycle")]
    Chain { name: String },
    #[error("style {name:?}: `extends` must be a string")]
    ExtendsType { name: String },
}

fn builtin_names() -> String {
    builtins().collect::<Vec<_>>().join(", ")
}

/// Names of the styles compiled into the binary.
pub fn builtins() -> impl Iterator<Item = &'static str> {
    BUILTIN.iter().map(|(n, _)| *n)
}

/// `~/.config/dat/styles`, where a user's own styles live.
#[must_use]
pub fn user_styles_dir() -> Option<PathBuf> {
    crate::paths::config_dir().map(|d| d.join("styles"))
}

/// Loads a style by name, resolving its `extends` chain.
pub fn load(name: &str, user_dir: Option<&Path>) -> Result<Style, StyleError> {
    let table = resolve(name, user_dir, 0)?;
    table.try_into().map_err(|e| StyleError::Parse {
        name: name.to_owned(),
        err: Box::new(e),
    })
}

fn resolve(name: &str, user_dir: Option<&Path>, depth: usize) -> Result<Table, StyleError> {
    if depth > MAX_CHAIN {
        return Err(StyleError::Chain { name: name.to_owned() });
    }
    let mut table = read(name, user_dir)?;
    let Some(parent) = table.remove("extends") else {
        return Ok(table);
    };
    let Value::String(parent) = parent else {
        return Err(StyleError::ExtendsType { name: name.to_owned() });
    };
    // A user file that extends its own name builds on the built-in it replaces.
    let parent_dir = if parent == name { None } else { user_dir };
    let mut merged = resolve(&parent, parent_dir, depth + 1)?;
    merge(&mut merged, table);
    Ok(merged)
}

fn read(name: &str, user_dir: Option<&Path>) -> Result<Table, StyleError> {
    let user_file = user_dir.map(|d| d.join(format!("{name}.toml")));
    let text = match user_file.as_deref().filter(|p| p.is_file()) {
        Some(path) => std::fs::read_to_string(path).map_err(|err| StyleError::Read {
            name: name.to_owned(),
            path: path.to_owned(),
            err,
        })?,
        None => BUILTIN
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, text)| (*text).to_owned())
            .ok_or_else(|| StyleError::NotFound {
                name: name.to_owned(),
                dir: user_dir.map_or_else(|| "no user dir".to_owned(), |d| d.display().to_string()),
            })?,
    };
    toml::from_str(&text).map_err(|e| StyleError::Parse {
        name: name.to_owned(),
        err: Box::new(e),
    })
}

/// Child keys win; tables merge recursively so a rule can override one field.
fn merge(base: &mut Table, child: Table) {
    for (key, value) in child {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(b)), Value::Table(c)) => merge(b, c),
            (_, v) => {
                base.insert(key, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{HeadingLine, TableLines};

    #[test]
    fn a_user_style_can_extend_the_built_in_it_replaces() {
        let dir = std::env::temp_dir().join(format!("dat-style-self-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("github.toml"), "extends = \"github\"\nh1 = { below = 3 }\n").expect("write");
        let s = load("github", Some(&dir)).expect("loads");
        assert_eq!((s.h1.below, s.h1.rule), (3, HeadingLine::Column));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn base_loads_in_full() {
        let s = load("base", None).expect("base parses");
        assert!(matches!(s.measure, crate::style::Measure::Full(_)));
        assert_eq!(s.h1.rule, HeadingLine::None);
    }

    #[test]
    fn github_overrides_only_what_it_names() {
        let s = load("github", None).expect("github parses");
        assert_eq!(s.h1.rule, HeadingLine::Column);
        assert_eq!(s.h1.above, 2, "inherited from base");
        assert_eq!(s.table.lines, TableLines::Box);
        assert!(s.callout.icon);
        assert_eq!(s.callout.bar, "▌");
        assert_eq!(s.callout.kinds.tip, crate::theme::Role::Done, "inherited");
    }

    #[test]
    fn unknown_key_is_an_error() {
        let dir = std::env::temp_dir().join(format!("dat-style-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("typo.toml"), "extends = \"github\"\nh1 = { bolt = true }\n").expect("write");
        let err = load("typo", Some(&dir)).expect_err("unknown field rejected");
        assert!(err.to_string().contains("bolt"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_style_names_the_builtins() {
        let err = load("nope", None).expect_err("missing");
        assert!(err.to_string().contains("github"));
    }
}
