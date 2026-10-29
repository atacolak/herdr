use std::path::{Path, PathBuf};

/// Resolves plain-text path candidates against the clicked pane's live cwd.
///
/// Hover runs on pointer moves, so construction only snapshots cheap values and
/// every check stays in-process: a plugin regex match plus at most two `stat`s.
pub(super) struct PathLinkResolver {
    cwd: Option<PathBuf>,
    home: Option<PathBuf>,
    patterns: Vec<regex::Regex>,
}

impl PathLinkResolver {
    pub(super) fn new(cwd: Option<PathBuf>, patterns: Vec<regex::Regex>) -> Self {
        Self {
            cwd,
            home: std::env::var_os("HOME").map(PathBuf::from),
            patterns,
        }
    }

    /// True when an installed plugin link handler matches `url`.
    pub(super) fn has_handler(&self, url: &str) -> bool {
        self.patterns.iter().any(|pattern| pattern.is_match(url))
    }

    /// True when a plugin link handler matches `candidate` and the candidate
    /// names something that exists relative to the pane cwd.
    pub(super) fn is_path_link(&self, candidate: &str) -> bool {
        self.has_handler(candidate) && self.resolved_path(candidate).is_some()
    }

    /// `candidate` may carry a `:line:col` or `#L<line>` suffix. Accept the
    /// literal token when it exists on disk, otherwise retry without the suffix.
    fn resolved_path(&self, candidate: &str) -> Option<PathBuf> {
        let direct = self.absolute_path(candidate);
        if direct.as_deref().is_some_and(Path::exists) {
            return direct;
        }
        let stripped = self.absolute_path(strip_line_suffix(candidate)?)?;
        stripped.exists().then_some(stripped)
    }

    fn absolute_path(&self, candidate: &str) -> Option<PathBuf> {
        let candidate = candidate.strip_prefix("file://").unwrap_or(candidate);
        if candidate.is_empty() {
            return None;
        }
        if let Some(rest) = candidate.strip_prefix("~/") {
            return Some(self.home.as_ref()?.join(rest));
        }
        let path = Path::new(candidate);
        if path.is_absolute() {
            return Some(path.to_path_buf());
        }
        Some(self.cwd.as_ref()?.join(path))
    }
}

/// Drops a trailing `#L12` or `:12` / `:12:3` editor position suffix.
fn strip_line_suffix(candidate: &str) -> Option<&str> {
    if let Some((head, line)) = candidate.rsplit_once("#L") {
        if !line.is_empty() && line.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(head);
        }
    }
    let mut rest = candidate;
    let mut stripped = false;
    for _ in 0..2 {
        let Some((head, tail)) = rest.rsplit_once(':') else {
            break;
        };
        if tail.is_empty() || !tail.chars().all(|ch| ch.is_ascii_digit()) {
            break;
        }
        rest = head;
        stripped = true;
    }
    stripped.then_some(rest)
}
