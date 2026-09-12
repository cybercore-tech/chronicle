mod config;
mod discover;
mod git;
mod report;
mod state;

use clap::Parser;
use report::{DigestKind, RepoDigest};
use std::process::{Command, ExitCode};

#[derive(Parser, Debug)]
#[command(name = "chronicle", version = "0.1.0", about = "Periodic activity digest for a tree of git repos")]
struct Args {
    /// Path to config.toml. Defaults to $XDG_CONFIG_HOME/chronicle/config.toml,
    /// then ~/.config/chronicle/config.toml, then ./config.toml.
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Disable cybercore color output in the terminal report.
    #[arg(long)]
    no_color: bool,

    /// Also list repos with no activity, not just the ones with something to report.
    #[arg(long)]
    show_unchanged: bool,
}

/// Shells out to `date` rather than adding a datetime dependency — same
/// "reuse the system tool" pattern as everywhere else this session.
fn today_and_now() -> (String, String) {
    let out = Command::new("date").arg("+%Y-%m-%d %H:%M:%S").output();
    let text = out.ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let mut parts = text.splitn(2, ' ');
    (parts.next().unwrap_or("unknown-date").to_string(), parts.next().unwrap_or("unknown-time").to_string())
}

fn append_markdown(dir: &std::path::Path, digests: &[RepoDigest], show_unchanged: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let (date, time) = today_and_now();
    let file_path = dir.join(format!("{date}.md"));

    let mut content = if file_path.exists() {
        std::fs::read_to_string(&file_path)?
    } else {
        format!("---\ntitle: \"Chronicle — {date}\"\ntags: [chronicle, git-activity]\n---\n\n# Chronicle — {date}\n\n")
    };

    content.push_str(&format!("## {time} run\n\n"));
    content.push_str(&report::render_markdown(digests, show_unchanged));
    content.push('\n');

    std::fs::write(&file_path, content)
}

fn main() -> ExitCode {
    let args = Args::parse();

    let config_path = match args.config.or_else(config::default_config_path) {
        Some(p) => p,
        None => {
            eprintln!("chronicle: no config found (checked --config, $XDG_CONFIG_HOME/chronicle, ~/.config/chronicle, ./config.toml)");
            return ExitCode::FAILURE;
        }
    };
    let cfg = match config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chronicle: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut repos = Vec::new();
    for root in &cfg.root {
        let root_path = config::expand_home(&root.path);
        repos.extend(discover::find_repos(&root_path, cfg.max_depth));
    }
    repos.sort();

    let state_path = config::expand_home(&cfg.state_path);
    let mut st = state::load(&state_path);

    let digests: Vec<RepoDigest> = repos
        .into_iter()
        .map(|repo| {
            let key = repo.to_string_lossy().into_owned();
            let new_head = git::head(&repo).unwrap_or_default();
            let old_head = st.repos.get(&key).cloned();

            let kind = match &old_head {
                None => DigestKind::FirstSeen,
                Some(old) if *old == new_head => DigestKind::NoChange,
                Some(old) => match git::log_range(&repo, old, &new_head) {
                    Some(commits) if !commits.is_empty() => DigestKind::NewCommits(commits),
                    _ => DigestKind::Diverged,
                },
            };

            if !new_head.is_empty() {
                st.repos.insert(key, new_head);
            }

            let dirty = git::dirty_status(&repo);
            let github_base = git::github_base_url(&repo);
            RepoDigest { path: repo, kind, dirty, github_base }
        })
        .collect();

    if let Err(e) = state::save(&st, &state_path) {
        eprintln!("chronicle: warning: failed to save state: {e}");
    }

    if let Some(dir) = &cfg.markdown_out_dir {
        let dir_path = config::expand_home(dir);
        if let Err(e) = append_markdown(&dir_path, &digests, args.show_unchanged) {
            eprintln!("chronicle: warning: failed to write markdown log: {e}");
        } else {
            println!("chronicle: appended digest to {}", dir_path.display());
        }
    }

    print!("{}", report::render(&digests, !args.no_color, args.show_unchanged));
    ExitCode::SUCCESS
}
