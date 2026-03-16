use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;

pub fn run(name: &str) -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;
    let ws = WorkspaceConfig::load(&root, name)?;
    let ws_dir = WorkspaceConfig::workspace_dir(&root, name);

    println!("Workspace: {}", name.bold());
    println!("Created:   {}", ws.workspace.created);
    println!("Path:      {}", ws_dir.display());
    println!();
    println!("Repos:");

    for (repo_name, repo_info) in &ws.repos {
        let wt_path = ws_dir.join(repo_name);

        if !wt_path.exists() {
            println!("  {}  {}  {}", repo_name.bold(), repo_info.branch.dimmed(), "MISSING".red());
            continue;
        }

        let default_branch = config.repos.get(repo_name)
            .map(|r| r.default_branch.as_str())
            .unwrap_or("main");

        let branch = git::current_branch(&wt_path).unwrap_or_else(|_| repo_info.branch.clone());
        let ahead = git::commits_ahead(&wt_path, default_branch).unwrap_or(0);
        let dirty = git::dirty_count(&wt_path).unwrap_or(0);

        let mut parts = Vec::new();
        if ahead > 0 {
            parts.push(format!("{} commits ahead", ahead).cyan().to_string());
        }
        if dirty > 0 {
            parts.push(format!("{} file{} modified", dirty, if dirty == 1 { "" } else { "s" }).yellow().to_string());
        }
        if parts.is_empty() {
            parts.push("clean".green().to_string());
        }

        println!("  {:<20} {:<35} {}", repo_name.bold(), branch.dimmed(), parts.join(", "));
    }

    Ok(())
}
