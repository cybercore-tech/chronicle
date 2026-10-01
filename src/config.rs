use serde::Deserialize;
use std::path::PathBuf;

fn default_state_path() -> String {
    "~/.local/state/chronicle/state.json".to_string()
}

fn default_max_depth() -> usize {
    3
}

#[derive(Deserialize, Debug, Clone)]
pub struct RootEntry {
    pub path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(default = "default_state_path")]
    pub state_path: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// If set, each run appends a dated section to a per-day markdown file
    /// in this directory, instead of (or alongside) the terminal report.
    #[serde(default)]
    pub markdown_out_dir: Option<String>,
    pub root: Vec<RootEntry>,
}

pub fn default_config_path() -> Option<PathBuf> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok()?;
    let candidates = [
        config_home.join("chronicle").join("config.toml"),
        PathBuf::from("config.toml"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn load(path: &std::path::Path) -> Result<AppConfig, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    toml::from_str(&raw).map_err(|e| format!("failed to parse {}: {}", path.display(), e))
}
