//! Finds git repositories under a root directory. Hand-rolled rather than
//! reaching for `walkdir` (already used elsewhere this session) because
//! the one rule that matters — stop descending the instant a `.git` is
//! found, never walk into a repo looking for repos nested inside it — is
//! simpler to express directly than to bolt onto a general-purpose walker.

use std::path::{Path, PathBuf};

pub fn find_repos(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, max_depth, 0, &mut out);
    out
}

fn walk(dir: &Path, max_depth: usize, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    if dir.join(".git").is_dir() {
        out.push(dir.to_path_buf());
        return; // a repo's own subdirectories are never searched for more repos
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            walk(&entry.path(), max_depth, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chronicle-test-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_a_top_level_repo() {
        let root = scratch("toplevel");
        std::fs::create_dir_all(root.join("myrepo/.git")).unwrap();

        let repos = find_repos(&root, 3);
        assert_eq!(repos, vec![root.join("myrepo")]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn does_not_descend_into_a_found_repo() {
        let root = scratch("nodescend");
        std::fs::create_dir_all(root.join("myrepo/.git")).unwrap();
        // a nested ".git"-like dir INSIDE the found repo must not be reported separately
        std::fs::create_dir_all(root.join("myrepo/vendor/other/.git")).unwrap();

        let repos = find_repos(&root, 5);
        assert_eq!(repos, vec![root.join("myrepo")]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn finds_multiple_sibling_repos() {
        let root = scratch("siblings");
        std::fs::create_dir_all(root.join("repo-a/.git")).unwrap();
        std::fs::create_dir_all(root.join("repo-b/.git")).unwrap();
        std::fs::create_dir_all(root.join("not-a-repo")).unwrap();

        let mut repos = find_repos(&root, 3);
        repos.sort();
        assert_eq!(repos, vec![root.join("repo-a"), root.join("repo-b")]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn respects_max_depth() {
        let root = scratch("depth");
        std::fs::create_dir_all(root.join("a/b/c/deeprepo/.git")).unwrap();

        assert!(find_repos(&root, 1).is_empty(), "repo is deeper than max_depth=1, should not be found");
        assert_eq!(find_repos(&root, 10), vec![root.join("a/b/c/deeprepo")]);
        std::fs::remove_dir_all(&root).ok();
    }
}
