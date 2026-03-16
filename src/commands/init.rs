use anyhow::{Result, bail};
use colored::Colorize;

use crate::config::{Config, ProjectConfig};

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Check if already initialized
    let ws_dir = cwd.join(".workspaces");
    if ws_dir.join("config.toml").exists() {
        bail!("Already initialized: {}", ws_dir.join("config.toml").display());
    }

    // Create .workspaces directory
    std::fs::create_dir_all(&ws_dir)?;

    // Auto-detect repos
    let repos = Config::detect_repos(&cwd);
    if repos.is_empty() {
        println!("{} No git repos detected in current directory.", "Warning:".yellow());
    }

    let config = Config {
        project: ProjectConfig {
            name: cwd.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string()),
            root: cwd.to_string_lossy().to_string(),
        },
        repos,
    };

    config.save(&cwd)?;

    println!("{} Initialized iws at {}", "✓".green(), ws_dir.display());
    println!("  Detected repos:");
    for (name, repo) in &config.repos {
        println!("    {} ({}, default: {})", name.bold(), repo.path, repo.default_branch);
    }

    Ok(())
}
