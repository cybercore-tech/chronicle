//! Shells out to `git` itself for everything — same reasoning as reusing
//! `ss`/`ufw`/`docker` in SentryGrid rather than re-deriving git's object
//! model from scratch, which would be a much bigger undertaking for no
//! real benefit here.

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct DirtyStatus {
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
}

impl DirtyStatus {
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && self.staged == 0 && self.untracked == 0
    }
}

pub fn head(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub full_hash: String,
    pub short_hash: String,
    pub subject: String,
}

/// Every commit in `old..new`, full + short hash and subject line kept
/// separate (rather than one pre-formatted string) so callers can build
/// either a plain display or a linked one. Empty if `old` isn't an
/// ancestor reachable this way (e.g. after a force-push or rebase rewrote
/// history) — that's reported by the caller as "history diverged" rather
/// than silently showing zero commits.
pub fn log_range(repo: &Path, old: &str, new: &str) -> Option<Vec<CommitInfo>> {
    let range = format!("{old}..{new}");
    // %x1f (unit separator) between fields, one commit per line — avoids
    // ambiguity with commit subjects that happen to contain spaces or colons.
    let output = Command::new("git")
        .args([
            "-C",
            &repo.to_string_lossy(),
            "log",
            "--format=%H%x1f%h%x1f%s",
            &range,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let commits = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\u{1f}');
            Some(CommitInfo {
                full_hash: parts.next()?.to_string(),
                short_hash: parts.next()?.to_string(),
                subject: parts.next().unwrap_or("").to_string(),
            })
        })
        .collect();
    Some(commits)
}

/// The `origin` remote, normalized to an `https://github.com/owner/repo`
/// base URL if it looks like a GitHub remote (SSH or HTTPS form) —
/// `None` for anything else (no remote, or a non-GitHub host), since
/// commit links are only buildable for GitHub.
pub fn github_base_url(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let path = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = url.strip_prefix("https://github.com/") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://github.com/") {
        rest
    } else {
        return None;
    };
    let path = path.strip_suffix(".git").unwrap_or(path);
    Some(format!("https://github.com/{path}"))
}

pub fn dirty_status(repo: &Path) -> DirtyStatus {
    let mut status = DirtyStatus::default();
    let Ok(output) = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "status", "--porcelain"])
        .output()
    else {
        return status;
    };
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // Porcelain format: two status characters, a space, then the path.
        // XY: X = index (staged) state, Y = worktree (unstaged) state.
        let mut chars = line.chars();
        let x = chars.next().unwrap_or(' ');
        let y = chars.next().unwrap_or(' ');
        if x == '?' && y == '?' {
            status.untracked += 1;
        } else {
            if x != ' ' {
                status.staged += 1;
            }
            if y != ' ' {
                status.modified += 1;
            }
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn init_repo(dir: &Path) {
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(dir)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test"])
            .current_dir(dir)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(dir)
            .status()
            .unwrap();
    }

    fn commit(dir: &Path, filename: &str, content: &str, message: &str) {
        std::fs::write(dir.join(filename), content).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-q", "-m", message])
            .current_dir(dir)
            .status()
            .unwrap();
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chronicle-git-test-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn head_returns_commit_hash() {
        let dir = scratch("head");
        init_repo(&dir);
        commit(&dir, "a.txt", "one", "first commit");
        let h = head(&dir).expect("head should resolve");
        assert_eq!(h.len(), 40, "should be a full sha1 hash");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn log_range_lists_commits_between_two_points() {
        let dir = scratch("range");
        init_repo(&dir);
        commit(&dir, "a.txt", "one", "first commit");
        let old = head(&dir).unwrap();
        commit(&dir, "a.txt", "two", "second commit");
        commit(&dir, "a.txt", "three", "third commit");
        let new = head(&dir).unwrap();

        let commits = log_range(&dir, &old, &new).expect("range should resolve");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "third commit");
        assert_eq!(commits[1].subject, "second commit");
        assert_eq!(
            commits[0].full_hash.len(),
            40,
            "full hash should be a complete sha1"
        );
        assert!(
            commits[0].full_hash.starts_with(&commits[0].short_hash),
            "short hash should be a prefix of the full one"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn github_base_url_normalizes_ssh_remote() {
        let dir = scratch("ssh-remote");
        init_repo(&dir);
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "git@github.com:darkstardevx/chronicle.git",
            ])
            .current_dir(&dir)
            .status()
            .unwrap();
        assert_eq!(
            github_base_url(&dir).as_deref(),
            Some("https://github.com/darkstardevx/chronicle")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn github_base_url_normalizes_https_remote() {
        let dir = scratch("https-remote");
        init_repo(&dir);
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/darkstardevx/chronicle.git",
            ])
            .current_dir(&dir)
            .status()
            .unwrap();
        assert_eq!(
            github_base_url(&dir).as_deref(),
            Some("https://github.com/darkstardevx/chronicle")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn github_base_url_none_without_a_remote() {
        let dir = scratch("no-remote");
        init_repo(&dir);
        assert_eq!(github_base_url(&dir), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dirty_status_counts_untracked_and_modified() {
        let dir = scratch("dirty");
        init_repo(&dir);
        commit(&dir, "a.txt", "one", "first commit");

        std::fs::write(dir.join("a.txt"), "modified content").unwrap();
        std::fs::write(dir.join("new.txt"), "brand new").unwrap();

        let status = dirty_status(&dir);
        assert_eq!(status.modified, 1);
        assert_eq!(status.untracked, 1);
        assert_eq!(status.staged, 0);
        assert!(!status.is_clean());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clean_repo_reports_clean() {
        let dir = scratch("clean");
        init_repo(&dir);
        commit(&dir, "a.txt", "one", "first commit");
        assert!(dirty_status(&dir).is_clean());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn staged_change_counts_separately_from_unstaged() {
        let dir = scratch("staged");
        init_repo(&dir);
        commit(&dir, "a.txt", "one", "first commit");

        std::fs::write(dir.join("a.txt"), "staged change").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&dir)
            .status()
            .unwrap();

        let status = dirty_status(&dir);
        assert_eq!(status.staged, 1);
        assert_eq!(status.modified, 0);
        std::fs::remove_dir_all(&dir).ok();
    }
}
