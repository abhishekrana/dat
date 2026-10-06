//! The user's `config.toml`: defaults for what the flags set. A missing file is all defaults; an unknown key is an error.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// File name inside `paths::config_dir()`.
pub const FILE: &str = "config.toml";

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Theme flavor; `DAT_THEME` and `--theme` override it.
    pub theme: Option<String>,
    /// Style name; `--style` overrides it.
    pub style: Option<String>,
    /// Follow the file on disk; `--no-watch` turns it off.
    pub watch: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config {path}: {err}")]
    Read { path: PathBuf, err: std::io::Error },
    #[error("config {path}: {err}")]
    Parse { path: PathBuf, err: Box<toml::de::Error> },
}

/// `config.toml` under the config dir, if there is a config dir.
#[must_use]
pub fn path() -> Option<PathBuf> {
    crate::paths::config_dir().map(|d| d.join(FILE))
}

/// Reads the file at `path`; a file that does not exist is the default config.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                err,
            });
        }
    };
    toml::from_str(&text).map_err(|e| ConfigError::Parse {
        path: path.to_owned(),
        err: Box::new(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(name: &str, text: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dat-config-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn a_missing_file_is_the_default() {
        let got = load(Path::new("/no/such/dir/config.toml")).unwrap();
        assert_eq!(got, Config::default());
    }

    #[test]
    fn every_key_parses() {
        let path = write("all", "theme = \"solarized-dark\"\nstyle = \"github\"\nwatch = false\n");
        let got = load(&path).unwrap();
        assert_eq!(got.theme.as_deref(), Some("solarized-dark"));
        assert_eq!(got.style.as_deref(), Some("github"));
        assert_eq!(got.watch, Some(false));
    }

    #[test]
    fn an_unknown_key_is_an_error_naming_the_file() {
        let path = write("unknown", "them = \"solarized-dark\"\n");
        let err = load(&path).unwrap_err().to_string();
        assert!(err.contains("config.toml") && err.contains("them"), "{err}");
    }
}
