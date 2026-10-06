//! Links: what following one means, and how a wikilink finds its note.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::doc::slug;
use crate::layout::Link;

/// How far below a vault root the wikilink search descends.
const WIKI_DEPTH: usize = 8;

/// What a link target asks for. Every heading is an id, as `doc::slug` makes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A note in the vault, by name, with an optional heading.
    Wiki { name: String, heading: Option<String> },
    /// A heading in this document.
    Anchor(String),
    /// A markdown file, relative to this one, with an optional heading.
    Note { path: PathBuf, heading: Option<String> },
    /// Anything else: handed to the desktop.
    External(String),
}

#[must_use]
pub fn classify(link: &Link) -> Action {
    let link = match link {
        Link::Wiki(target) => return classify_wiki(target),
        Link::Href(href) => href.as_str(),
    };
    if let Some(id) = link.strip_prefix('#') {
        return Action::Anchor(id.to_owned());
    }
    if link.contains("://") || link.starts_with("mailto:") {
        return Action::External(link.to_owned());
    }
    let (path, heading) = split_heading(link);
    if Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    {
        return Action::Note {
            path: PathBuf::from(percent_decode(path)),
            heading,
        };
    }
    Action::External(link.to_owned())
}

/// `[[Note#Heading]]` names a heading by its text; `[[#Heading]]` is one in this document.
fn classify_wiki(target: &str) -> Action {
    let (name, heading) = split_heading(target);
    let heading = heading.map(|h| slug(&h));
    match heading {
        Some(id) if name.is_empty() => Action::Anchor(id),
        heading => Action::Wiki {
            name: name.to_owned(),
            heading,
        },
    }
}

/// Decodes `%XX` escapes, as in a link to `My%20Note.md`; text that does not decode stays as written.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[i] == b'%'
            && let (Some(hi), Some(lo)) = (
                bytes.get(i + 1).copied().and_then(hex),
                bytes.get(i + 2).copied().and_then(hex),
            )
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

fn split_heading(target: &str) -> (&str, Option<String>) {
    match target.split_once('#') {
        Some((base, h)) if !h.is_empty() => (base, Some(h.to_owned())),
        _ => (target, None),
    }
}

/// The note file a wikilink names: beside the current file, else anywhere under the vault root.
#[must_use]
pub fn resolve_wiki(name: &str, from: Option<&Path>) -> Option<PathBuf> {
    let dir = from.map_or_else(|| std::env::current_dir().unwrap_or_default(), Path::to_path_buf);
    let want = format!("{name}.md");
    let beside = dir.join(&want);
    if beside.is_file() {
        return Some(beside);
    }
    let root = vault_root(&dir).unwrap_or(dir);
    find_file(&root, &want, WIKI_DEPTH)
}

/// The nearest ancestor holding `.obsidian` or `.git`.
fn vault_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| d.join(".obsidian").is_dir() || d.join(".git").exists())
        .map(Path::to_path_buf)
}

/// Depth-first search for a file name, case-insensitive, skipping hidden directories.
fn find_file(dir: &Path, want: &str, depth: usize) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && name != "node_modules" {
                dirs.push(path);
            }
        } else if name.eq_ignore_ascii_case(want) {
            return Some(path);
        }
    }
    if depth == 0 {
        return None;
    }
    dirs.sort();
    dirs.into_iter().find_map(|d| find_file(&d, want, depth - 1))
}

/// Hands a URL or path to the desktop, detached from the terminal; a thread reaps it so no zombie is left.
pub fn open_external(target: &str) -> std::io::Result<()> {
    let mut child = Command::new("xdg-open")
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_classify_by_shape() {
        let href = |s: &str| Link::Href(s.to_owned());
        let wiki = |s: &str| Link::Wiki(s.to_owned());
        assert_eq!(
            classify(&wiki("Server layout#Disks & RAID")),
            Action::Wiki {
                name: "Server layout".into(),
                heading: Some("disks-raid".into())
            }
        );
        assert_eq!(classify(&wiki("#What's new?")), Action::Anchor("whats-new".into()));
        assert_eq!(classify(&href("#schedule")), Action::Anchor("schedule".into()));
        assert_eq!(
            classify(&href("wiki:Foo")),
            Action::External("wiki:Foo".into()),
            "a markdown link is never a wikilink"
        );
        assert_eq!(
            classify(&href("My%20Notes/caf%C3%A9.md#top")),
            Action::Note {
                path: "My Notes/café.md".into(),
                heading: Some("top".into())
            }
        );
        assert_eq!(
            classify(&href("50%.md")),
            Action::Note {
                path: "50%.md".into(),
                heading: None
            }
        );
        assert_eq!(
            classify(&href("notes/other.md")),
            Action::Note {
                path: "notes/other.md".into(),
                heading: None
            }
        );
        assert!(matches!(
            classify(&href("https://example.com/a.md")),
            Action::External(_)
        ));
        assert!(matches!(classify(&href("image.png")), Action::External(_)));
    }

    #[test]
    fn wikilinks_resolve_beside_the_file_then_under_the_vault_root() {
        let root = std::env::temp_dir().join(format!("dat-wiki-{}", std::process::id()));
        std::fs::create_dir_all(root.join(".obsidian")).expect("root");
        std::fs::create_dir_all(root.join("a/deep")).expect("dirs");
        std::fs::create_dir_all(root.join("b")).expect("dirs");
        std::fs::write(root.join("a/Here.md"), "").expect("write");
        std::fs::write(root.join("b/Far Away.md"), "").expect("write");
        let from = root.join("a/deep");
        assert_eq!(
            resolve_wiki("Here", Some(&root.join("a"))),
            Some(root.join("a/Here.md"))
        );
        assert_eq!(resolve_wiki("far away", Some(&from)), Some(root.join("b/Far Away.md")));
        assert_eq!(resolve_wiki("Missing", Some(&from)), None);
        std::fs::remove_dir_all(&root).ok();
    }
}
