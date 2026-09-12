use crate::git::DirtyStatus;
use std::path::PathBuf;

pub enum DigestKind {
    /// No prior state for this repo — tracking starts now, nothing to
    /// report yet (avoids dumping the entire pre-existing commit history
    /// on the very first run, same reasoning as SigilWard's `init` not
    /// complaining that every file is "new").
    FirstSeen,
    NoChange,
    NewCommits(Vec<String>),
    /// The previously-recorded HEAD isn't a resolvable ancestor of the
    /// current one — a rebase, amend, or history rewrite happened, so a
    /// simple `old..new` commit list isn't meaningful.
    Diverged,
}

pub struct RepoDigest {
    pub path: PathBuf,
    pub kind: DigestKind,
    pub dirty: DirtyStatus,
}

fn color(kind: &DigestKind) -> String {
    match kind {
        DigestKind::FirstSeen => cybercore::palette::cyan(),
        DigestKind::NoChange => cybercore::palette::white(),
        DigestKind::NewCommits(_) => cybercore::palette::acid_green(),
        DigestKind::Diverged => cybercore::palette::orange(),
    }
}

fn reset() -> &'static str {
    cybercore::palette::RESET
}

pub fn render(digests: &[RepoDigest], color_on: bool, show_unchanged: bool) -> String {
    let mut out = String::new();
    let mut active = 0;

    for d in digests {
        let quiet = matches!(d.kind, DigestKind::NoChange) && d.dirty.is_clean();
        if quiet && !show_unchanged {
            continue;
        }
        if !quiet {
            active += 1;
        }

        let (c, r) = if color_on { (color(&d.kind), reset().to_string()) } else { (String::new(), String::new()) };
        let name = d.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| d.path.display().to_string());

        out.push_str(&format!("{c}=== {name} ==={r}\n"));
        match &d.kind {
            DigestKind::FirstSeen => out.push_str("  tracking started\n"),
            DigestKind::NoChange => out.push_str("  no new commits\n"),
            DigestKind::Diverged => out.push_str("  history rewritten since last check (rebase/amend/force-push) — no linear commit list available\n"),
            DigestKind::NewCommits(commits) => {
                for line in commits {
                    out.push_str(&format!("  {line}\n"));
                }
            }
        }
        if !d.dirty.is_clean() {
            out.push_str(&format!(
                "  working tree: {} modified, {} staged, {} untracked\n",
                d.dirty.modified, d.dirty.staged, d.dirty.untracked
            ));
        }
    }

    out.push_str(&format!("\n{} repo(s) total, {} with activity to report\n", digests.len(), active));
    out
}

/// Plain-markdown rendering (no ANSI color) — one `###` section per repo
/// with activity, meant to be appended under a `## HH:MM:SS run` heading
/// in a dated vault note rather than printed to a terminal.
pub fn render_markdown(digests: &[RepoDigest], show_unchanged: bool) -> String {
    let mut out = String::new();
    let mut active = 0;

    for d in digests {
        let quiet = matches!(d.kind, DigestKind::NoChange) && d.dirty.is_clean();
        if quiet && !show_unchanged {
            continue;
        }
        if !quiet {
            active += 1;
        }

        let name = d.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| d.path.display().to_string());
        out.push_str(&format!("### {name}\n\n"));
        match &d.kind {
            DigestKind::FirstSeen => out.push_str("tracking started\n\n"),
            DigestKind::NoChange => out.push_str("no new commits\n\n"),
            DigestKind::Diverged => out.push_str("history rewritten since last check (rebase/amend/force-push) — no linear commit list available\n\n"),
            DigestKind::NewCommits(commits) => {
                for line in commits {
                    out.push_str(&format!("- `{line}`\n"));
                }
                out.push('\n');
            }
        }
        if !d.dirty.is_clean() {
            out.push_str(&format!(
                "*working tree: {} modified, {} staged, {} untracked*\n\n",
                d.dirty.modified, d.dirty.staged, d.dirty.untracked
            ));
        }
    }

    if active == 0 {
        out.push_str("_No activity since last check._\n");
    }
    out
}
