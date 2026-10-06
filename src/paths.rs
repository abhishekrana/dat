//! XDG base directories for dat. A variable that is empty or relative counts as unset, as the spec says.

use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/dat`, else `~/.config/dat`: `config.toml` and `styles/`.
#[must_use]
pub fn config_dir() -> Option<PathBuf> {
    base("XDG_CONFIG_HOME", ".config").map(|d| d.join("dat"))
}

/// `$XDG_STATE_HOME/dat`, else `~/.local/state/dat`: logs and reading positions.
#[must_use]
pub fn state_dir() -> Option<PathBuf> {
    base("XDG_STATE_HOME", ".local/state").map(|d| d.join("dat"))
}

fn base(var: &str, under_home: &str) -> Option<PathBuf> {
    resolve(
        std::env::var_os(var).map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
        under_home,
    )
}

fn resolve(xdg: Option<PathBuf>, home: Option<PathBuf>, under_home: &str) -> Option<PathBuf> {
    xdg.filter(|p| p.is_absolute())
        .or_else(|| home.filter(|h| h.is_absolute()).map(|h| h.join(under_home)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_wins_when_absolute() {
        let got = resolve(Some("/x".into()), Some("/home/u".into()), ".config");
        assert_eq!(got, Some(PathBuf::from("/x")));
    }

    #[test]
    fn empty_or_relative_xdg_falls_back_to_home() {
        for xdg in ["", "rel/dir"] {
            let got = resolve(Some(xdg.into()), Some("/home/u".into()), ".config");
            assert_eq!(got, Some(PathBuf::from("/home/u/.config")), "{xdg:?}");
        }
    }

    #[test]
    fn nothing_usable_is_none() {
        assert_eq!(resolve(None, None, ".config"), None);
        assert_eq!(
            resolve(Some(String::new().into()), Some(String::new().into()), ".config"),
            None
        );
    }
}
