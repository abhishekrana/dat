//! Copying: the session's clipboard tool - wl-copy, xclip or pbcopy - else OSC 52 through the terminal.

use std::io::Write as _;
use std::process::{Command, Stdio};

/// What carried the text to the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    Tool(&'static str),
    /// OSC 52: works where the terminal allows it, e.g. tmux with `set-clipboard on`.
    Terminal,
}

#[derive(Debug, thiserror::Error)]
pub enum CopyError {
    #[error("{tool}: {err}")]
    Run { tool: &'static str, err: std::io::Error },
    #[error("{tool} exited with {status}")]
    Failed {
        tool: &'static str,
        status: std::process::ExitStatus,
    },
    #[error("cannot write to the terminal: {0}")]
    Terminal(std::io::Error),
}

/// Copies `text` with the first tool this session can use, else through the terminal.
pub fn copy(text: &str) -> Result<Via, CopyError> {
    for (tool, args) in tools() {
        match Command::new(tool)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                let wrote = child.stdin.take().map(|mut s| s.write_all(text.as_bytes()));
                let status = child.wait().map_err(|err| CopyError::Run { tool, err })?;
                if let Some(Err(err)) = wrote {
                    return Err(CopyError::Run { tool, err });
                }
                return if status.success() {
                    Ok(Via::Tool(tool))
                } else {
                    Err(CopyError::Failed { tool, status })
                };
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(CopyError::Run { tool, err }),
        }
    }
    let mut out = std::io::stdout().lock();
    out.write_all(osc52(text).as_bytes())
        .and_then(|()| out.flush())
        .map_err(CopyError::Terminal)?;
    Ok(Via::Terminal)
}

/// The tools worth trying here: a display server's own, or macOS's.
fn tools() -> Vec<(&'static str, &'static [&'static str])> {
    let set = |var: &str| std::env::var_os(var).is_some_and(|v| !v.is_empty());
    let mut tools: Vec<(&'static str, &'static [&'static str])> = Vec::new();
    if set("WAYLAND_DISPLAY") {
        tools.push(("wl-copy", &[]));
    }
    if set("DISPLAY") {
        tools.push(("xclip", &["-selection", "clipboard"]));
    }
    if cfg!(target_os = "macos") {
        tools.push(("pbcopy", &[]));
    }
    tools
}

fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, encoded) in cases {
            assert_eq!(base64(plain.as_bytes()), encoded, "{plain:?}");
        }
    }

    #[test]
    fn osc52_sets_the_clipboard_selection() {
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
