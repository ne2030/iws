use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::workspace::WorkspaceConfig;
use crate::git;

pub fn run() -> Result<()> {
    let root = Config::find_root()?;
    let config = Config::load(&root)?;
    let workspaces = WorkspaceConfig::list_all(&root)?;

    if workspaces.is_empty() {
        println!("No workspaces found. Create one with: iws new <name> --repos <repo1,repo2>");
        return Ok(());
    }

    for ws_name in &workspaces {
        let ws = WorkspaceConfig::load(&root, ws_name)?;
        let ws_dir = WorkspaceConfig::workspace_dir(&root, ws_name);

        let mut statuses = Vec::new();
        for (repo_name, repo_info) in &ws.repos {
            let wt_path = ws_dir.join(repo_name);
            if !wt_path.exists() {
                statuses.push(format!("{}: {}", repo_name, "missing".red()));
                continue;
            }

            let default_branch = config.repos.get(repo_name)
                .map(|r| r.default_branch.as_str())
                .unwrap_or("main");

            let ahead = git::commits_ahead(&wt_path, default_branch).unwrap_or(0);
            let dirty = git::dirty_count(&wt_path).unwrap_or(0);

            let status = match (ahead, dirty) {
                (0, 0) => "clean".green().to_string(),
                (a, 0) => format!("{}↑", a).cyan().to_string(),
                (0, d) => format!("{}✎", d).yellow().to_string(),
                (a, d) => format!("{}↑ {}✎", a, d).yellow().to_string(),
            };
            let _ = repo_info; // branch info available if needed
            statuses.push(format!("{}: {}", repo_name, status));
        }

        println!("  {}  [{}]", ws_name.bold(), statuses.join(", "));
    }

    Ok(())
}
